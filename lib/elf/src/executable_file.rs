use address::VirtualAddress;

use crate::{ElfError, ElfFile, FileType, Machine, Segment, SegmentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableError<E> {
    Elf(ElfError),
    NotExecutable(FileType),
    WrongMachine(Machine),
    SegmentsOverlap,
    EntryOutsideSegments,
    Placement(E),
}

impl<E> From<ElfError> for ExecutableError<E> {
    fn from(error: ElfError) -> Self {
        Self::Elf(error)
    }
}

/// An ELF file checked as an executable for one processor: every loadable
/// segment passed the caller's placement rule, no two of them overlap, and
/// the entry point lies in one of them
#[derive(Clone, Copy)]
pub struct ExecutableFile<'a> {
    file: ElfFile<'a>,
    virtual_base: VirtualAddress,
    interpreter: Option<&'a [u8]>,
    dynamic: bool,
}

impl<'a> ExecutableFile<'a> {
    /// A loadable segment of no memory is left out of every check and every
    /// listing; an interpreter or a dynamic linking table is reported, not
    /// refused
    pub fn new<E>(
        file: ElfFile<'a>,
        machine: Machine,
        placement: impl Fn(&Segment<'_>) -> Result<(), E>,
    ) -> Result<Self, ExecutableError<E>> {
        let header = file.header();
        if header.file_type != FileType::EXECUTABLE {
            return Err(ExecutableError::NotExecutable(header.file_type));
        }
        if header.machine != machine {
            return Err(ExecutableError::WrongMachine(header.machine));
        }
        let mut interpreter = None;
        let mut dynamic = false;
        let mut lowest: Option<VirtualAddress> = None;
        let mut entry_inside_segments = false;
        for (index, segment) in file.segments().enumerate() {
            let segment = segment?;
            match segment.kind() {
                SegmentKind::INTERPRETER => interpreter = Some(segment.bytes()),
                SegmentKind::DYNAMIC => dynamic = true,
                _ => {}
            }
            if !loads_memory(&segment) {
                continue;
            }
            placement(&segment).map_err(ExecutableError::Placement)?;
            if overlaps_an_earlier_segment(&file, index, &segment)? {
                return Err(ExecutableError::SegmentsOverlap);
            }
            let address = segment.virtual_address();
            lowest = Some(lowest.map_or(address, |lowest| lowest.min(address)));
            entry_inside_segments |= segment.contains(header.entry);
        }
        match lowest {
            Some(virtual_base) if entry_inside_segments => Ok(Self {
                file,
                virtual_base,
                interpreter,
                dynamic,
            }),
            _ => Err(ExecutableError::EntryOutsideSegments),
        }
    }

    pub fn entry(&self) -> VirtualAddress {
        self.file.header().entry
    }

    /// The lowest address any segment occupies
    pub fn virtual_base(&self) -> VirtualAddress {
        self.virtual_base
    }

    /// The loadable segments that occupy memory, in file order
    pub fn segments(&self) -> impl Iterator<Item = Result<Segment<'a>, ElfError>> + use<'a> {
        self.file.segments().filter_map(|segment| {
            segment
                .map(|segment| loads_memory(&segment).then_some(segment))
                .transpose()
        })
    }

    /// As the file holds it, with its terminating zero
    pub fn interpreter(&self) -> Option<&'a [u8]> {
        self.interpreter
    }

    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }
}

fn loads_memory(segment: &Segment<'_>) -> bool {
    segment.kind() == SegmentKind::LOAD && segment.memory_size() != 0
}

fn overlaps_an_earlier_segment(
    file: &ElfFile<'_>,
    index: usize,
    segment: &Segment<'_>,
) -> Result<bool, ElfError> {
    for earlier in file.segments().take(index) {
        let earlier = earlier?;
        if loads_memory(&earlier)
            && earlier.virtual_address() < segment.end()
            && segment.virtual_address() < earlier.end()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::*;
    use crate::program_header::{EXECUTE, READ, WRITE};
    use crate::testing::{IMAGE_BASE, Image};

    fn kernel() -> Image {
        Image::new()
            .header(|header| header.entry = VirtualAddress::new(IMAGE_BASE + 0x10))
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE,
                &[0x90; 32],
                32,
                READ | EXECUTE,
            )
            .segment(SegmentKind::LOAD, IMAGE_BASE + 0x1000, &[0xaa; 8], 8, READ)
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE + 0x2000,
                &[1, 2, 3],
                0x100,
                READ | WRITE,
            )
    }

    fn anywhere(_: &Segment<'_>) -> Result<(), ()> {
        Ok(())
    }

    fn executable(bytes: &[u8]) -> Result<ExecutableFile<'_>, ExecutableError<()>> {
        ExecutableFile::new(ElfFile::parse(bytes).unwrap(), Machine::X86_64, anywhere)
    }

    fn loadable_segments<'a>(executable: &ExecutableFile<'a>) -> Vec<Segment<'a>> {
        executable.segments().collect::<Result<_, _>>().unwrap()
    }

    #[test]
    fn accepts_a_static_executable() {
        let bytes = kernel().build();
        let executable = executable(&bytes).unwrap();
        assert_eq!(executable.entry(), VirtualAddress::new(IMAGE_BASE + 0x10));
        assert_eq!(executable.virtual_base(), VirtualAddress::new(IMAGE_BASE));
        let segments = loadable_segments(&executable);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[1].bytes(), [0xaa; 8]);
        assert_eq!(segments[2].bytes(), [1, 2, 3]);
        assert_eq!(segments[2].memory_size(), 0x100);
        assert!(segments[2].is_writable() && !segments[2].is_executable());
        assert_eq!(executable.interpreter(), None);
        assert!(!executable.is_dynamic());
    }

    #[test]
    fn virtual_base_is_the_lowest_address_in_any_order() {
        let bytes = Image::new()
            .header(|header| header.entry = VirtualAddress::new(IMAGE_BASE + 0x1000))
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE + 0x1000,
                &[0x90; 8],
                8,
                READ | EXECUTE,
            )
            .segment(SegmentKind::LOAD, IMAGE_BASE, &[0xaa; 8], 8, READ)
            .build();
        let executable = executable(&bytes).unwrap();
        assert_eq!(executable.virtual_base(), VirtualAddress::new(IMAGE_BASE));
    }

    #[test]
    fn rejects_a_shared_object() {
        let bytes = kernel().header(|header| header.file_type = 3).build();
        assert_eq!(
            executable(&bytes).err(),
            Some(ExecutableError::NotExecutable(FileType::SHARED_OBJECT))
        );
    }

    #[test]
    fn rejects_a_foreign_machine() {
        let bytes = kernel().build();
        let file = ElfFile::parse(&bytes).unwrap();
        assert_eq!(
            ExecutableFile::new(file, Machine::AARCH64, anywhere).err(),
            Some(ExecutableError::WrongMachine(Machine::X86_64))
        );
    }

    #[test]
    fn elf_errors_come_through() {
        let mut bytes = kernel().build();
        bytes.truncate(bytes.len() - 1);
        assert_eq!(
            executable(&bytes).err(),
            Some(ExecutableError::Elf(ElfError::Truncated))
        );
    }

    #[test]
    fn placement_errors_come_back_as_placement() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct BelowTheImage(VirtualAddress);

        let bytes = kernel()
            .segment(SegmentKind::LOAD, 0x40_0000, &[0; 4], 4, READ)
            .build();
        let above_the_image = |segment: &Segment<'_>| {
            if segment.virtual_address() < VirtualAddress::new(IMAGE_BASE) {
                Err(BelowTheImage(segment.virtual_address()))
            } else {
                Ok(())
            }
        };
        let file = ElfFile::parse(&bytes).unwrap();
        assert_eq!(
            ExecutableFile::new(file, Machine::X86_64, above_the_image).err(),
            Some(ExecutableError::Placement(BelowTheImage(
                VirtualAddress::new(0x40_0000)
            )))
        );
        let kernel = kernel().build();
        let file = ElfFile::parse(&kernel).unwrap();
        assert!(ExecutableFile::new(file, Machine::X86_64, above_the_image).is_ok());
    }

    #[test]
    fn rejects_overlapping_segments() {
        let bytes = kernel()
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE + 0x10,
                &[0; 4],
                4,
                READ | WRITE,
            )
            .build();
        assert_eq!(
            executable(&bytes).err(),
            Some(ExecutableError::SegmentsOverlap)
        );

        let adjacent = kernel()
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE + 0x20,
                &[0; 4],
                4,
                READ | WRITE,
            )
            .build();
        assert!(executable(&adjacent).is_ok());
    }

    #[test]
    fn rejects_an_entry_outside_the_segments() {
        let bytes = kernel()
            .header(|header| header.entry = VirtualAddress::new(IMAGE_BASE + 0x20))
            .build();
        assert_eq!(
            executable(&bytes).err(),
            Some(ExecutableError::EntryOutsideSegments)
        );

        let no_segments = Image::new().build();
        assert_eq!(
            executable(&no_segments).err(),
            Some(ExecutableError::EntryOutsideSegments)
        );
    }

    #[test]
    fn a_segment_of_no_memory_is_left_out() {
        let bytes = kernel()
            .segment(SegmentKind::LOAD, IMAGE_BASE + 0x10, &[], 0, READ | WRITE)
            .build();
        let executable = executable(&bytes).unwrap();
        assert_eq!(loadable_segments(&executable).len(), 3);
    }

    #[test]
    fn segments_borrow_the_bytes_not_the_executable() {
        fn segments_of(bytes: &[u8]) -> impl Iterator<Item = Result<Segment<'_>, ElfError>> {
            let executable = executable(bytes).unwrap();
            executable.segments()
        }

        let bytes = kernel().build();
        assert_eq!(
            segments_of(&bytes)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn segments_are_read_again_from_the_bytes() {
        let bytes = kernel().build();
        let executable = executable(&bytes).unwrap();
        let first: Vec<Result<Segment<'_>, ElfError>> = executable.segments().collect();
        let second: Vec<Result<Segment<'_>, ElfError>> = executable.segments().collect();
        assert_eq!(first, second);
    }

    #[test]
    fn reports_interpreter_and_dynamic_table() {
        let bytes = kernel()
            .segment(SegmentKind::INTERPRETER, 0, b"/ld.so\0", 7, READ)
            .segment(
                SegmentKind::DYNAMIC,
                IMAGE_BASE + 0x3000,
                &[0; 16],
                16,
                READ | WRITE,
            )
            .build();
        let executable = executable(&bytes).unwrap();
        assert_eq!(executable.interpreter(), Some(&b"/ld.so\0"[..]));
        assert!(executable.is_dynamic());
        assert_eq!(loadable_segments(&executable).len(), 3);
    }
}

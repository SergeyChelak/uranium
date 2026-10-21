use address::VirtualAddress;

use crate::{ElfError, ElfFile, FileType, Machine, Segment, SegmentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableError<E> {
    /// Not a well-formed ELF file
    Elf(ElfError),
    /// Not an executable file; carries the type the file claims
    NotExecutable(FileType),
    /// Built for another processor; carries the one the file names
    WrongMachine(Machine),
    /// Two loadable segments claim the same memory
    SegmentsOverlap,
    /// The entry point lies in no loadable segment
    EntryOutsideSegments,
    /// The caller's placement rule refused a segment
    Placement(E),
}

impl<E> From<ElfError> for ExecutableError<E> {
    fn from(error: ElfError) -> Self {
        Self::Elf(error)
    }
}

/// An ELF file checked as an executable for one processor: every loadable
/// segment passed the caller's placement rule, no two of them overlap, and
/// the entry point lies in one of them. The segments are read from the
/// file again whenever asked for, as the program header table already holds
/// them; nothing is copied
#[derive(Clone, Copy)]
pub struct ExecutableFile<'a> {
    file: ElfFile<'a>,
}

impl<'a> ExecutableFile<'a> {
    /// Checks `file` as an executable for `machine`. `placement` is the
    /// caller's rule for where a loadable segment may lie, and its error
    /// comes back as `Placement`. A loadable segment of no memory is left
    /// out of every check and every listing: there is nothing to load or
    /// map. An interpreter or a dynamic linking table is reported, not
    /// refused: whether the caller can honour them is the caller's decision
    pub fn new<E>(
        file: ElfFile<'a>,
        machine: Machine,
        placement: impl Fn(&Segment) -> Result<(), E>,
    ) -> Result<Self, ExecutableError<E>> {
        let header = file.header();
        if header.file_type != FileType::EXECUTABLE {
            return Err(ExecutableError::NotExecutable(header.file_type));
        }
        if header.machine != machine {
            return Err(ExecutableError::WrongMachine(header.machine));
        }
        let mut entry_inside_segments = false;
        for (index, segment) in file.segments().enumerate() {
            let segment = segment?;
            if !loads_memory(&segment) {
                continue;
            }
            placement(&segment).map_err(ExecutableError::Placement)?;
            if overlaps_an_earlier_segment(&file, index, &segment)? {
                return Err(ExecutableError::SegmentsOverlap);
            }
            entry_inside_segments |= segment.contains(header.entry);
        }
        if !entry_inside_segments {
            return Err(ExecutableError::EntryOutsideSegments);
        }
        Ok(Self { file })
    }

    pub fn entry(&self) -> VirtualAddress {
        self.file.header().entry
    }

    /// The lowest address any segment occupies
    pub fn virtual_base(&self) -> VirtualAddress {
        self.segments()
            .map(|segment| segment.virtual_address)
            .min()
            .expect("an executable has a segment: the one holding its entry")
    }

    /// The loadable segments that occupy memory, in file order
    pub fn segments(&self) -> impl Iterator<Item = Segment> + 'a {
        self.entries().filter(loads_memory)
    }

    /// The bytes the file holds for `segment`; the rest of its memory is
    /// zero
    pub fn segment_bytes(&self, segment: &Segment) -> &'a [u8] {
        self.file.segment_bytes(segment)
    }

    /// The path of the program interpreter, as the file holds it with its
    /// terminating zero, when the file asks for one
    pub fn interpreter(&self) -> Option<&'a [u8]> {
        self.entries()
            .find(|segment| segment.kind == SegmentKind::INTERPRETER)
            .map(|segment| self.file.segment_bytes(&segment))
    }

    /// Whether the file carries a dynamic linking table
    pub fn is_dynamic(&self) -> bool {
        self.entries()
            .any(|segment| segment.kind == SegmentKind::DYNAMIC)
    }

    /// Every entry of the program header table; `new` read each of them
    /// once already, so none can fail now
    fn entries(&self) -> impl Iterator<Item = Segment> + 'a {
        self.file
            .segments()
            .map(|segment| segment.expect("every entry was read when the executable was made"))
    }
}

/// Whether an entry is a loadable segment with memory to load into
fn loads_memory(segment: &Segment) -> bool {
    segment.kind == SegmentKind::LOAD && segment.memory_size != 0
}

/// Whether `segment`, the entry at `index`, shares memory with a loadable
/// segment before it in the table
fn overlaps_an_earlier_segment(
    file: &ElfFile<'_>,
    index: usize,
    segment: &Segment,
) -> Result<bool, ElfError> {
    for earlier in file.segments().take(index) {
        let earlier = earlier?;
        if loads_memory(&earlier)
            && earlier.virtual_address < segment.end()
            && segment.virtual_address < earlier.end()
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

    /// The three segments the kernel's linker script lays out: code,
    /// constants, and data with a zeroed tail, with the entry in the code
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

    fn anywhere(_: &Segment) -> Result<(), ()> {
        Ok(())
    }

    fn executable(bytes: &[u8]) -> Result<ExecutableFile<'_>, ExecutableError<()>> {
        ExecutableFile::new(ElfFile::parse(bytes).unwrap(), Machine::X86_64, anywhere)
    }

    #[test]
    fn accepts_a_static_executable() {
        let bytes = kernel().build();
        let executable = executable(&bytes).unwrap();
        assert_eq!(executable.entry(), VirtualAddress::new(IMAGE_BASE + 0x10));
        assert_eq!(executable.virtual_base(), VirtualAddress::new(IMAGE_BASE));
        let segments: Vec<Segment> = executable.segments().collect();
        assert_eq!(segments.len(), 3);
        assert_eq!(executable.segment_bytes(&segments[1]), [0xaa; 8]);
        assert_eq!(executable.segment_bytes(&segments[2]), [1, 2, 3]);
        assert_eq!(segments[2].memory_size, 0x100);
        assert!(segments[2].writable && !segments[2].executable);
        assert_eq!(executable.interpreter(), None);
        assert!(!executable.is_dynamic());
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
        let above_the_image = |segment: &Segment| {
            if segment.virtual_address < VirtualAddress::new(IMAGE_BASE) {
                Err(BelowTheImage(segment.virtual_address))
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
        assert_eq!(executable.segments().count(), 3);
    }

    #[test]
    fn segments_are_read_again_from_the_bytes() {
        let bytes = kernel().build();
        let executable = executable(&bytes).unwrap();
        let first: Vec<Segment> = executable.segments().collect();
        let second: Vec<Segment> = executable.segments().collect();
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
        assert_eq!(executable.segments().count(), 3);
    }
}

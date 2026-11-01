use crate::{Frame, Page, PageSize, Size2MiB, Size4KiB};

macro_rules! range_type {
    ($name:ident, $iterator:ident, $page:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub struct $name<S: PageSize = Size4KiB> {
            start: $page<S>,
            count: u64,
        }

        impl<S: PageSize> $name<S> {
            /// `None` when the range would reach past the top of the address space
            pub fn new(start: $page<S>, count: u64) -> Option<Self> {
                let pages_in_space = u64::MAX / S::SIZE + 1;
                let fits = start
                    .number()
                    .checked_add(count)
                    .is_some_and(|end| end <= pages_in_space);
                fits.then_some(Self { start, count })
            }

            pub fn start(self) -> $page<S> {
                self.start
            }

            pub fn len(self) -> u64 {
                self.count
            }

            pub fn is_empty(self) -> bool {
                self.count == 0
            }

            pub fn contains(self, page: $page<S>) -> bool {
                page.number()
                    .checked_sub(self.start.number())
                    .is_some_and(|offset| offset < self.count)
            }
        }

        impl<S: PageSize> IntoIterator for $name<S> {
            type Item = $page<S>;
            type IntoIter = $iterator<S>;

            fn into_iter(self) -> $iterator<S> {
                $iterator {
                    next: self.start.number(),
                    remaining: self.count,
                    size: core::marker::PhantomData,
                }
            }
        }

        #[derive(Clone, Debug)]
        pub struct $iterator<S: PageSize> {
            next: u64,
            remaining: u64,
            size: core::marker::PhantomData<S>,
        }

        impl<S: PageSize> Iterator for $iterator<S> {
            type Item = $page<S>;

            fn next(&mut self) -> Option<$page<S>> {
                if self.remaining == 0 {
                    return None;
                }
                let page = $page::from_number(self.next);
                self.next += 1;
                self.remaining -= 1;
                Some(page)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let remaining = usize::try_from(self.remaining).ok();
                (remaining.unwrap_or(usize::MAX), remaining)
            }
        }
    };
}

range_type!(PageRange, PageIterator, Page);
range_type!(FrameRange, FrameIterator, Frame);

/// The large page lies within the address space, so its range is built
/// without the check
macro_rules! covers {
    ($range:ident, $page:ident, $large:ident, $small:ident) => {
        impl From<$page<$large>> for $range<$small> {
            fn from(large: $page<$large>) -> Self {
                const FACTOR: u64 = $large::SIZE / $small::SIZE;
                Self {
                    start: $page::from_number(large.number() * FACTOR),
                    count: FACTOR,
                }
            }
        }

        impl TryFrom<$range<$small>> for $page<$large> {
            type Error = $range<$small>;

            fn try_from(range: $range<$small>) -> Result<Self, Self::Error> {
                const FACTOR: u64 = $large::SIZE / $small::SIZE;
                let aligned = range.start().number() % FACTOR == 0;
                if aligned && range.len() == FACTOR {
                    Ok($page::from_number(range.start().number() / FACTOR))
                } else {
                    Err(range)
                }
            }
        }
    };
}

covers!(PageRange, Page, Size2MiB, Size4KiB);
covers!(FrameRange, Frame, Size2MiB, Size4KiB);

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::{FrameRange, PageRange};
    use crate::{Frame, Page, PhysicalAddress, Size2MiB, Size4KiB, VirtualAddress};

    const PAGES_IN_SPACE: u64 = 1 << 52;

    fn page(address: u64) -> Page<Size4KiB> {
        Page::containing(VirtualAddress::new(address))
    }

    #[test]
    fn iterates_every_page() {
        let range = PageRange::new(page(0x1000), 3).unwrap();
        let pages: Vec<Page> = range.into_iter().collect();
        assert_eq!(pages, [page(0x1000), page(0x2000), page(0x3000)]);
        assert_eq!(range.len(), 3);
        assert_eq!(range.into_iter().size_hint(), (3, Some(3)));
    }

    #[test]
    fn contains_excludes_the_end() {
        let range = PageRange::new(page(0x1000), 3).unwrap();
        assert!(!range.contains(page(0x0)));
        assert!(range.contains(page(0x1000)));
        assert!(range.contains(page(0x3fff)));
        assert!(!range.contains(page(0x4000)));
    }

    #[test]
    fn an_empty_range_contains_nothing() {
        let range = PageRange::new(page(0x1000), 0).unwrap();
        assert!(range.is_empty());
        assert!(!range.contains(page(0x1000)));
        assert_eq!(range.into_iter().next(), None);
    }

    #[test]
    fn iteration_reaches_the_last_page_of_the_space() {
        let last = page(u64::MAX);
        let pages: Vec<Page> = PageRange::new(last, 1).unwrap().into_iter().collect();
        assert_eq!(pages, [last]);
        assert_eq!(
            last.start_address(),
            VirtualAddress::new(0xffff_ffff_ffff_f000)
        );
    }

    #[test]
    fn a_range_ending_at_the_top_of_the_space_is_accepted() {
        assert!(PageRange::new(page(0), PAGES_IN_SPACE).is_some());
        assert!(PageRange::new(page(u64::MAX), 1).is_some());
    }

    #[test]
    fn a_range_reaching_past_the_top_of_the_space_is_refused() {
        assert_eq!(PageRange::new(page(0), PAGES_IN_SPACE + 1), None);
        assert_eq!(PageRange::new(page(u64::MAX), 2), None);
        assert_eq!(PageRange::new(page(u64::MAX), u64::MAX), None);
    }

    #[test]
    fn a_large_page_covers_512_small_ones() {
        let large = Page::<Size2MiB>::containing(VirtualAddress::new(0x40_0000));
        let small = PageRange::<Size4KiB>::from(large);
        assert_eq!(small.start(), page(0x40_0000));
        assert_eq!(small.len(), 512);
        assert!(small.contains(page(0x5f_f000)));
        assert!(!small.contains(page(0x60_0000)));
    }

    #[test]
    fn a_full_aligned_small_range_is_a_large_page() {
        let small = PageRange::new(page(0x40_0000), 512).unwrap();
        assert_eq!(
            Page::<Size2MiB>::try_from(small),
            Ok(Page::containing(VirtualAddress::new(0x40_0000)))
        );
    }

    #[test]
    fn unaligned_small_range_is_not_a_large_page() {
        let small = PageRange::new(page(0x40_1000), 512).unwrap();
        assert_eq!(Page::<Size2MiB>::try_from(small), Err(small));
    }

    #[test]
    fn a_short_small_range_is_not_a_large_page() {
        let small = PageRange::new(page(0x40_0000), 511).unwrap();
        assert_eq!(Page::<Size2MiB>::try_from(small), Err(small));
    }

    #[test]
    fn frame_ranges_behave_the_same() {
        let first = Frame::<Size4KiB>::containing(PhysicalAddress::new(0x7fff_0000));
        let range = FrameRange::new(first, 2).unwrap();
        let frames: Vec<Frame> = range.into_iter().collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].start_address(), PhysicalAddress::new(0x7fff_1000));
        let last = Frame::<Size4KiB>::containing(PhysicalAddress::new(u64::MAX));
        assert_eq!(FrameRange::new(last, 2), None);
        let large = Frame::<Size2MiB>::containing(PhysicalAddress::new(0x20_0000));
        assert_eq!(FrameRange::<Size4KiB>::from(large).len(), 512);
    }
}

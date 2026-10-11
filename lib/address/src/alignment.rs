pub(crate) fn align_down(value: u64, alignment: u64) -> u64 {
    assert!(
        alignment.is_power_of_two(),
        "alignment must be a power of two"
    );
    value & !(alignment - 1)
}

pub(crate) fn align_up(value: u64, alignment: u64) -> u64 {
    let aligned_down = align_down(value, alignment);
    if aligned_down == value {
        value
    } else {
        aligned_down
            .checked_add(alignment)
            .expect("aligning up overflows the address space")
    }
}

pub(crate) fn is_aligned(value: u64, alignment: u64) -> bool {
    align_down(value, alignment) == value
}

#[cfg(test)]
mod tests {
    use super::{align_down, align_up, is_aligned};

    #[test]
    fn aligns_down_to_the_previous_multiple() {
        assert_eq!(align_down(0x1234, 0x1000), 0x1000);
        assert_eq!(align_down(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn aligns_up_to_the_next_multiple() {
        assert_eq!(align_up(0x1234, 0x1000), 0x2000);
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn checks_alignment() {
        assert!(is_aligned(0x2000, 0x1000));
        assert!(!is_aligned(0x2001, 0x1000));
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn rejects_non_power_of_two() {
        align_down(0x1234, 0x1001);
    }

    #[test]
    #[should_panic(expected = "overflows")]
    fn align_up_panics_on_overflow() {
        align_up(u64::MAX, 0x1000);
    }
}

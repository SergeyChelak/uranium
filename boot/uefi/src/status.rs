use core::fmt;

// UEFI 2.11, Appendix D: error codes have the highest bit set.
const ERROR_BIT: usize = 1 << (usize::BITS - 1);

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(usize);

macro_rules! known_status_codes {
    ($($name:ident = $value:expr),* $(,)?) => {
        impl Status {
            $(pub const $name: Status = Status($value);)*
        }

        const KNOWN: &[(Status, &str)] = &[$((Status::$name, stringify!($name))),*];
    };
}

known_status_codes! {
    // UEFI 2.11, Appendix D, Table D.2
    SUCCESS = 0,
    // UEFI 2.11, Appendix D, Table D.3
    LOAD_ERROR = ERROR_BIT | 1,
    INVALID_PARAMETER = ERROR_BIT | 2,
    UNSUPPORTED = ERROR_BIT | 3,
    BUFFER_TOO_SMALL = ERROR_BIT | 5,
    NOT_READY = ERROR_BIT | 6,
    DEVICE_ERROR = ERROR_BIT | 7,
    OUT_OF_RESOURCES = ERROR_BIT | 9,
    NOT_FOUND = ERROR_BIT | 14,
    TIMEOUT = ERROR_BIT | 18,
    ABORTED = ERROR_BIT | 21,
}

impl Status {
    pub const fn is_error(self) -> bool {
        self.0 & ERROR_BIT != 0
    }

    pub fn into_result(self) -> Result<(), Status> {
        if self.is_error() { Err(self) } else { Ok(()) }
    }

    pub const fn raw(self) -> usize {
        self.0
    }

    fn name(self) -> Option<&'static str> {
        KNOWN
            .iter()
            .find(|(status, _)| *status == self)
            .map(|(_, name)| *name)
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => formatter.write_str(name),
            None => write!(formatter, "{:#x}", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ERROR_BIT, KNOWN, Status};
    use std::format;

    #[test]
    fn success_is_zero_and_not_an_error() {
        assert_eq!(Status::SUCCESS.raw(), 0);
        assert!(!Status::SUCCESS.is_error());
    }

    #[test]
    fn error_codes_have_the_high_bit_set() {
        for (status, name) in KNOWN
            .iter()
            .filter(|(status, _)| *status != Status::SUCCESS)
        {
            assert!(status.is_error(), "{name} must be an error");
            assert_ne!(
                status.raw() & ERROR_BIT,
                0,
                "{name} must have the high bit set"
            );
        }
        assert_eq!(Status::LOAD_ERROR.raw(), ERROR_BIT | 1);
        assert_eq!(Status::NOT_FOUND.raw(), ERROR_BIT | 14);
        assert_eq!(Status::ABORTED.raw(), ERROR_BIT | 21);
    }

    #[test]
    fn into_result_maps_success_and_errors() {
        assert_eq!(Status::SUCCESS.into_result(), Ok(()));
        assert_eq!(Status::NOT_FOUND.into_result(), Err(Status::NOT_FOUND));
    }

    #[test]
    fn warnings_are_not_errors() {
        let unknown_glyph = Status(1);
        assert!(!unknown_glyph.is_error());
        assert_eq!(unknown_glyph.into_result(), Ok(()));
    }

    #[test]
    fn debug_prints_the_name_of_a_known_code_or_the_number() {
        assert_eq!(format!("{:?}", Status::SUCCESS), "SUCCESS");
        assert_eq!(format!("{:?}", Status::NOT_FOUND), "NOT_FOUND");
        let unknown = Status(ERROR_BIT | 0x20);
        assert_eq!(format!("{unknown:?}"), format!("{:#x}", ERROR_BIT | 0x20));
    }
}

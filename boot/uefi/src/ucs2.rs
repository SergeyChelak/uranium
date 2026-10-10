use core::str::Chars;

/// Code units per chunk handed to the firmware, not counting the terminator
pub(crate) const CHUNK_LENGTH: usize = 128;

const CARRIAGE_RETURN: u16 = b'\r' as u16;
const LINE_FEED: u16 = b'\n' as u16;
const REPLACEMENT: u16 = b'?' as u16;

/// Turns text into null-terminated UCS-2 chunks for the text console:
/// a line feed becomes carriage return plus line feed unless the carriage
/// return is already there, and characters outside the basic plane, which
/// UCS-2 cannot represent, become a question mark
pub(crate) struct Encoder<'a> {
    text: Chars<'a>,
    pending: Option<u16>,
    after_carriage_return: bool,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            text: text.chars(),
            pending: None,
            after_carriage_return: false,
        }
    }

    /// Fills `buffer` with the next chunk and a terminating zero. Returns the
    /// number of code units before the terminator, or `None` once the text
    /// is exhausted
    pub(crate) fn next_chunk(&mut self, buffer: &mut [u16; CHUNK_LENGTH + 1]) -> Option<usize> {
        let mut length = 0;
        while length < CHUNK_LENGTH {
            let Some(unit) = self.pending.take().or_else(|| self.next_unit()) else {
                break;
            };
            buffer[length] = unit;
            length += 1;
        }
        buffer[length] = 0;
        (length > 0).then_some(length)
    }

    fn next_unit(&mut self) -> Option<u16> {
        let character = self.text.next()?;
        let unit = match character {
            '\n' if !self.after_carriage_return => {
                self.pending = Some(LINE_FEED);
                CARRIAGE_RETURN
            }
            _ => u16::try_from(u32::from(character)).unwrap_or(REPLACEMENT),
        };
        self.after_carriage_return = unit == CARRIAGE_RETURN;
        Some(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_LENGTH, Encoder};
    use std::string::String;
    use std::vec::Vec;

    fn encode(text: &str) -> Vec<Vec<u16>> {
        let mut encoder = Encoder::new(text);
        let mut buffer = [u16::MAX; CHUNK_LENGTH + 1];
        let mut chunks = Vec::new();
        while let Some(length) = encoder.next_chunk(&mut buffer) {
            assert_eq!(buffer[length], 0, "chunk must be null-terminated");
            chunks.push(buffer[..length].to_vec());
        }
        chunks
    }

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn encodes_ascii_with_terminator() {
        assert_eq!(encode("hi"), [units("hi")]);
    }

    #[test]
    fn expands_newline_to_carriage_return_line_feed() {
        assert_eq!(encode("a\nb\n"), [units("a\r\nb\r\n")]);
    }

    #[test]
    fn keeps_existing_carriage_return() {
        assert_eq!(encode("a\r\nb"), [units("a\r\nb")]);
    }

    #[test]
    fn encodes_basic_plane_characters_as_one_unit() {
        assert_eq!(encode("уран"), [units("уран")]);
    }

    #[test]
    fn replaces_characters_outside_the_basic_plane() {
        assert_eq!(encode("a😀b"), [units("a?b")]);
    }

    #[test]
    fn splits_long_text_into_chunks() {
        let text: String = "x".repeat(CHUNK_LENGTH * 2 + 3);
        let chunks = encode(&text);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), CHUNK_LENGTH);
        assert_eq!(chunks[1].len(), CHUNK_LENGTH);
        assert_eq!(chunks[2], units("xxx"));
    }

    #[test]
    fn carries_the_line_feed_into_the_next_chunk() {
        let text = "x".repeat(CHUNK_LENGTH - 1) + "\ny";
        let chunks = encode(&text);
        assert_eq!(chunks[0].len(), CHUNK_LENGTH);
        assert_eq!(chunks[0][CHUNK_LENGTH - 1], u16::from(b'\r'));
        assert_eq!(chunks[1], units("\ny"));
    }

    #[test]
    fn empty_text_gives_no_chunk() {
        assert!(encode("").is_empty());
    }
}

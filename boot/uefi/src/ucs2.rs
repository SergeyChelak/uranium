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

/// Writes `text` with a terminating zero into `buffer`, the way the firmware
/// takes file names: no line-ending rules and no replacements. `None` when a
/// character lies outside the basic plane or the text does not fit
pub(crate) fn encode_terminated<'a>(text: &str, buffer: &'a mut [u16]) -> Option<&'a [u16]> {
    let mut length = 0;
    for character in text.chars() {
        *buffer.get_mut(length)? = u16::try_from(u32::from(character)).ok()?;
        length += 1;
    }
    *buffer.get_mut(length)? = 0;
    Some(&buffer[..=length])
}

/// Little-endian UCS-2 up to the first zero unit, written as UTF-8; the
/// number of bytes written, or `None` when the text does not fit
pub fn decode_terminated(bytes: &[u8], buffer: &mut [u8]) -> Option<usize> {
    let (pairs, _odd_trailing_byte) = bytes.as_chunks::<2>();
    let mut length = 0;
    for &pair in pairs {
        let unit = u16::from_le_bytes(pair);
        if unit == 0 {
            break;
        }
        let character = char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER);
        let encoded = buffer.get_mut(length..length + character.len_utf8())?;
        character.encode_utf8(encoded);
        length += encoded.len();
    }
    Some(length)
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_LENGTH, Encoder, decode_terminated, encode_terminated};
    use std::string::String;
    use std::vec;
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

    #[test]
    fn encode_terminated_appends_the_terminator() {
        let mut buffer = [u16::MAX; 4];
        assert_eq!(
            encode_terminated("ab", &mut buffer),
            Some(&units("ab\0")[..])
        );
    }

    #[test]
    fn encode_terminated_keeps_line_feeds_as_they_are() {
        let mut buffer = [u16::MAX; 4];
        assert_eq!(
            encode_terminated("a\nb", &mut buffer),
            Some(&units("a\nb\0")[..])
        );
    }

    #[test]
    fn encode_terminated_needs_room_for_the_terminator() {
        let mut buffer = [u16::MAX; 3];
        assert_eq!(encode_terminated("abc", &mut buffer), None);
        assert_eq!(
            encode_terminated("ab", &mut buffer),
            Some(&units("ab\0")[..])
        );
    }

    #[test]
    fn encode_terminated_rejects_characters_outside_the_basic_plane() {
        let mut buffer = [u16::MAX; 8];
        assert_eq!(encode_terminated("a😀", &mut buffer), None);
    }

    fn bytes(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn decode(bytes: &[u8], capacity: usize) -> Option<String> {
        let mut buffer = vec![0xff; capacity];
        let length = decode_terminated(bytes, &mut buffer)?;
        Some(String::from_utf8(buffer[..length].to_vec()).unwrap())
    }

    #[test]
    fn decode_terminated_decodes_ascii_options() {
        let options = bytes("kernel.image = \\URANIUM\\TEST.ELF");
        assert_eq!(
            decode(&options, 64).as_deref(),
            Some("kernel.image = \\URANIUM\\TEST.ELF")
        );
    }

    #[test]
    fn decode_terminated_stops_at_the_terminator() {
        assert_eq!(decode(&bytes("ab\0cd"), 16).as_deref(), Some("ab"));
    }

    #[test]
    fn decode_terminated_drops_an_odd_trailing_byte() {
        let mut options = bytes("ab");
        options.push(b'c');
        assert_eq!(decode(&options, 16).as_deref(), Some("ab"));
    }

    #[test]
    fn decode_terminated_writes_basic_plane_characters_as_utf8() {
        assert_eq!(decode(&bytes("уран"), 16).as_deref(), Some("уран"));
    }

    #[test]
    fn decode_terminated_replaces_a_lone_surrogate() {
        let mut options = bytes("a");
        options.extend_from_slice(&0xd800_u16.to_le_bytes());
        assert_eq!(decode(&options, 16).as_deref(), Some("a\u{fffd}"));
    }

    #[test]
    fn decode_terminated_fails_when_the_options_do_not_fit() {
        assert_eq!(decode(&bytes("abcd"), 3), None);
        assert_eq!(decode(&bytes("abcd"), 4).as_deref(), Some("abcd"));
        assert_eq!(decode(&bytes("уран"), 7), None);
    }

    #[test]
    fn decode_terminated_of_nothing_is_empty() {
        assert_eq!(decode(&[], 4).as_deref(), Some(""));
        assert_eq!(decode(&[0, 0], 0).as_deref(), Some(""));
    }
}

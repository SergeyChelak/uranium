use address::PhysicalAddress;

use crate::PhysicalRange;
use crate::layout::layout;

#[repr(C)]
pub(crate) struct RawFramebuffer {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: u32,
    pub(crate) pixel_format: u32,
}

layout!(RawFramebuffer, 32, {
    base: 0,
    size: 8,
    width: 16,
    height: 20,
    stride: 24,
    pixel_format: 28,
});

/// The byte order of a pixel in the framebuffer; the discriminant is the
/// code that travels in the wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Unknown = 0,
    Rgb8 = 1,
    Bgr8 = 2,
}

impl From<PixelFormat> for u32 {
    fn from(format: PixelFormat) -> u32 {
        format as u32
    }
}

impl From<u32> for PixelFormat {
    fn from(code: u32) -> Self {
        match code {
            1 => Self::Rgb8,
            2 => Self::Bgr8,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Framebuffer {
    pub range: PhysicalRange,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
}

impl Framebuffer {
    pub(crate) fn from_raw(raw: &RawFramebuffer) -> Option<Self> {
        let range = PhysicalRange::new(raw.base, raw.size)?;
        Some(Self {
            range,
            width: raw.width,
            height: raw.height,
            stride: raw.stride,
            pixel_format: PixelFormat::from(raw.pixel_format),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pixel_format_round_trips_through_u32() {
        for format in [PixelFormat::Unknown, PixelFormat::Rgb8, PixelFormat::Bgr8] {
            assert_eq!(PixelFormat::from(u32::from(format)), format);
        }
    }

    #[test]
    fn unknown_pixel_format_is_unknown() {
        for code in [3, u32::MAX] {
            assert_eq!(PixelFormat::from(code), PixelFormat::Unknown);
        }
    }
}

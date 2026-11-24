//! YAML character encoding detection and decode (§5.2).

use crate::error::{Error, Result};

/// Decode a YAML byte stream to a UTF-8 [`String`].
///
/// Detects a leading BOM for UTF-8 / UTF-16LE / UTF-16BE / UTF-32LE / UTF-32BE.
/// Without a BOM, bytes are treated as UTF-8. The BOM itself is not part of the
/// returned character stream.
pub(crate) fn decode_yaml_bytes(input: &[u8]) -> Result<String> {
    if input.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        decode_utf32(&input[4..], true)
    } else if input.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        decode_utf32(&input[4..], false)
    } else if input.starts_with(&[0xFE, 0xFF]) {
        decode_utf16(&input[2..], true)
    } else if input.starts_with(&[0xFF, 0xFE]) {
        decode_utf16(&input[2..], false)
    } else if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        decode_utf8(&input[3..])
    } else {
        decode_utf8(input)
    }
}

fn decode_utf8(input: &[u8]) -> Result<String> {
    std::str::from_utf8(input)
        .map(str::to_owned)
        .map_err(|e| Error::InvalidUtf8 {
            valid_up_to: e.valid_up_to(),
        })
}

fn decode_utf16(input: &[u8], big_endian: bool) -> Result<String> {
    if input.len() % 2 != 0 {
        return Err(Error::InvalidEncoding {
            message: "UTF-16 input length is not a multiple of 2".into(),
        });
    }
    let units: Vec<u16> = input
        .chunks_exact(2)
        .map(|c| {
            if big_endian {
                u16::from_be_bytes([c[0], c[1]])
            } else {
                u16::from_le_bytes([c[0], c[1]])
            }
        })
        .collect();
    String::from_utf16(&units).map_err(|_| Error::InvalidEncoding {
        message: "invalid UTF-16 sequence".into(),
    })
}

fn decode_utf32(input: &[u8], big_endian: bool) -> Result<String> {
    if input.len() % 4 != 0 {
        return Err(Error::InvalidEncoding {
            message: "UTF-32 input length is not a multiple of 4".into(),
        });
    }
    let mut out = String::with_capacity(input.len() / 4);
    for chunk in input.chunks_exact(4) {
        let code = if big_endian {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        let ch = char::from_u32(code).ok_or_else(|| Error::InvalidEncoding {
            message: format!("invalid UTF-32 code point U+{code:X}"),
        })?;
        out.push(ch);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf16le_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for u in "hi".encode_utf16() {
            bytes.extend(u.to_le_bytes());
        }
        assert_eq!(decode_yaml_bytes(&bytes).unwrap(), "hi");
    }

    #[test]
    fn utf32le_bom_beats_utf16le_prefix() {
        let mut bytes = vec![0xFF, 0xFE, 0x00, 0x00];
        bytes.extend(u32::to_le_bytes('x' as u32));
        assert_eq!(decode_yaml_bytes(&bytes).unwrap(), "x");
    }
}

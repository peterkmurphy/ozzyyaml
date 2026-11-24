//! YAML character encoding detection and decode (§5.2).

use crate::error::{Error, Result};

/// Decode a YAML byte stream to a UTF-8 [`String`].
///
/// Detects encoding per YAML 1.2.2 §5.2: BOM rows first, then null-pattern
/// rows, else UTF-8. A leading BOM is not part of the returned character stream.
pub(crate) fn decode_yaml_bytes(input: &[u8]) -> Result<String> {
    // Explicit BOMs (longest / most specific first).
    if input.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return decode_utf32(&input[4..], true);
    }
    if input.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return decode_utf32(&input[4..], false);
    }
    if input.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&input[2..], true);
    }
    if input.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&input[2..], false);
    }
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return decode_utf8(&input[3..]);
    }

    // Null-pattern detection when there is no BOM.
    if input.len() >= 4 {
        if input[0] == 0x00 && input[1] == 0x00 && input[2] == 0x00 {
            // `00 00 00 xx` → UTF-32BE
            return decode_utf32(input, true);
        }
        if input[1] == 0x00 && input[2] == 0x00 && input[3] == 0x00 {
            // `xx 00 00 00` → UTF-32LE
            return decode_utf32(input, false);
        }
    }
    if input.len() >= 2 {
        if input[0] == 0x00 {
            // `00 xx` → UTF-16BE
            return decode_utf16(input, true);
        }
        if input[1] == 0x00 {
            // `xx 00` → UTF-16LE
            return decode_utf16(input, false);
        }
    }

    decode_utf8(input)
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

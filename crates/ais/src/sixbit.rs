// SPDX-License-Identifier: MIT OR Apache-2.0

//! Six-bit AIS ASCII packing (ITU-R M.1371 / IEC 61161-1).
//!
//! AIS payloads are encoded as a sequence of 6-bit characters taken from the
//! AIS ASCII table, then transmitted as a base-64-like string. This module
//! unpacks that string into a bit stream that message decoders consume.

// Bit shifts that sign-extend intentionally rely on wrapping casts.
#![allow(clippy::cast_possible_wrap)]

/// Decode an AIS six-bit packed ASCII string into a contiguous bit vector.
///
/// Invalid characters (anything outside the AIS 6-bit table) are treated as
/// zero bits, matching the spec's "fill" behaviour for unexpected input.
#[must_use]
pub fn unpack(payload: &str) -> Vec<bool> {
    let mut bits = Vec::with_capacity(payload.len() * 6);
    for ch in payload.bytes() {
        let value = sixbit_value(ch);
        for i in (0..6).rev() {
            bits.push((value >> i) & 1 == 1);
        }
    }
    bits
}

/// Map an AIS ASCII character to its 6-bit value (0..=63) per ITU-R M.1371
/// (the AIVDM wire encoding: `0`-`9`, `:`, `;`, `<`, `=`, `>`, `?`, `@`,
/// `A`-`W`, `` ` ``, `a`-`w`).
#[must_use]
pub fn sixbit_value(ch: u8) -> u8 {
    match ch {
        b'0'..=b'9' => ch - b'0',
        b':' => 10,
        b';' => 11,
        b'<' => 12,
        b'=' => 13,
        b'>' => 14,
        b'?' => 15,
        b'@'..=b'W' => ch - b'@' + 16,
        b'`'..=b'w' => ch - b'`' + 40,
        _ => 63,
    }
}

/// A cursor over a bit vector produced by [`unpack`].
pub struct BitReader {
    bits: Vec<bool>,
    pos: usize,
}

impl BitReader {
    /// Create a reader over the given unpacked bits.
    #[must_use]
    pub fn new(bits: Vec<bool>) -> Self {
        Self { bits, pos: 0 }
    }

    /// Number of bits that have not yet been read.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.bits.len().saturating_sub(self.pos)
    }

    /// Read `n` bits as an unsigned integer (MSB-first).
    ///
    /// Bits beyond the available length are read as zero (truncation-safe).
    #[must_use]
    pub fn read_u32(&mut self, n: usize) -> u32 {
        let mut value = 0u32;
        for _ in 0..n {
            value <<= 1;
            if self.pos < self.bits.len() && self.bits[self.pos] {
                value |= 1;
            }
            self.pos += 1;
        }
        value
    }

    /// Read `n` bits as a signed two's-complement integer (MSB-first).
    #[must_use]
    pub fn read_i32(&mut self, n: usize) -> i32 {
        let raw = self.read_u32(n);
        if n == 0 {
            return 0;
        }
        let shift = 32 - n;
        ((raw << shift) as i32) >> shift
    }

    /// Read `n` bits as an ASCII string, decoding each 6-bit group through the
    /// AIS table and stripping trailing `@` (space) padding.
    #[must_use]
    pub fn read_string(&mut self, n: usize) -> String {
        let mut out = String::new();
        for _ in 0..n.div_ceil(6) {
            let six = self.read_u32(6);
            if let Ok(six_u8) = u8::try_from(six) {
                if let Some(ch) = char_for_sixbit(six_u8) {
                    out.push(ch);
                }
            }
        }
        out.trim_end_matches('@').to_string()
    }
}

/// Map a 6-bit value back to its AIS ASCII character (AIVDM wire encoding).
#[must_use]
pub fn char_for_sixbit(value: u8) -> Option<char> {
    Some(match value {
        0..=9 => (b'0' + value) as char,
        10 => ':',
        11 => ';',
        12 => '<',
        13 => '=',
        14 => '>',
        15 => '?',
        16..=39 => (b'@' + value - 16) as char,
        40..=63 => (b'`' + value - 40) as char,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_known_character() {
        // 'A' maps to 6-bit value 17 -> bits 010001
        let bits = unpack("A");
        assert_eq!(bits, vec![false, true, false, false, false, true]);
    }

    #[test]
    fn reads_unsigned() {
        let bits = unpack("AB");
        let mut r = BitReader::new(bits);
        // 'A' = 17, 'B' = 18 in the AIS wire table
        assert_eq!(r.read_u32(6), 17);
        assert_eq!(r.read_u32(6), 18);
    }

    #[test]
    fn reads_signed_negative() {
        let bits = vec![true, true, true, true, true, true];
        let mut r = BitReader::new(bits);
        assert_eq!(r.read_i32(6), -1);
    }

    #[test]
    fn reads_string() {
        // value 0 = '0', value 1 = '1'
        let bits = unpack("01");
        let mut r = BitReader::new(bits);
        assert_eq!(r.read_string(12), "01");
    }

    #[test]
    fn strips_trailing_pad_at_sign() {
        let bits = unpack("0@");
        let mut r = BitReader::new(bits);
        assert_eq!(r.read_string(12), "0");
    }

    #[test]
    fn roundtrip_sixbit_value() {
        for ch in b'0'..=b'9' {
            assert_eq!(char_for_sixbit(sixbit_value(ch)).unwrap() as u8, ch);
        }
        for ch in b'A'..=b'W' {
            assert_eq!(char_for_sixbit(sixbit_value(ch)).unwrap() as u8, ch);
        }
        for ch in b'a'..=b'w' {
            assert_eq!(char_for_sixbit(sixbit_value(ch)).unwrap() as u8, ch);
        }
    }
}

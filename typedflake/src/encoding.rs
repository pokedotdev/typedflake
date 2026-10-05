//! Compact text form of IDs in a configurable alphabet.
//!
//! Everything here is pure, and decoding ends in the same validation as every
//! other constructor.

use core::fmt;
use core::ops::Deref;

use crate::id::{self, Id, InvalidId, Repr};

/// Characters an alphabet may use: printable ASCII without the space.
const FIRST_CHAR: u8 = b'!';
const LAST_CHAR: u8 = b'~';
const MAX_CHARS: usize = (LAST_CHAR - FIRST_CHAR) as usize + 1;

/// Marks a byte that is not in the alphabet.
const NO_DIGIT: u8 = u8::MAX;

/// Longest encoding: a 64-bit value in a two-character alphabet.
const MAX_ENCODED_LEN: usize = 64;

/// The characters an ID is written with, one per digit.
///
/// The number of characters is the base: [`BASE62`](Self::BASE62) writes IDs
/// in base 62. Give an ID type an alphabet with the `alphabet` option of
/// [`#[typedflake]`](macro@crate::typedflake) to get `encode` and `decode`:
///
/// ```
/// use typedflake::{Alphabet, typedflake};
///
/// // Digits and lowercase letters, without the easily confused `0`, `1`, `i`, `l`, and `o`.
/// pub const FRIENDLY: Alphabet = Alphabet::new("23456789abcdefghjkmnpqrstuvwxyz");
///
/// #[typedflake(epoch = "2025-01-01", alphabet = FRIENDLY)]
/// pub struct InviteId(i64);
///
/// let id = InviteId::try_from(232_900_560_974_681_078_i64)?;
/// let text = id.encode();
///
/// assert_eq!(text.as_str(), "2b76swcw35egs");
/// assert_eq!(InviteId::decode(&text)?, id);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Every ID of a type encodes to the same number of characters. When the
/// alphabet lists its characters in ascending ASCII order, as the presets do,
/// encoded IDs sort the same way the IDs do.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Alphabet {
    chars: [u8; MAX_CHARS],
    base: u8,
    /// Digit of each ASCII byte, or `NO_DIGIT`.
    digits: [u8; 128],
}

impl Alphabet {
    /// Digits, then lowercase letters.
    pub const BASE36: Self = Self::new("0123456789abcdefghijklmnopqrstuvwxyz");

    /// The Bitcoin alphabet: no `0`, `O`, `I`, or `l`.
    pub const BASE58: Self =
        Self::new("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz");

    /// Digits, uppercase letters, then lowercase letters.
    pub const BASE62: Self =
        Self::new("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");

    /// The 64 URL-safe characters of base64url, in ascending ASCII order so
    /// that encoded IDs stay sortable.
    ///
    /// This is not the base64 encoding of the ID's bytes, and a base64 decoder
    /// will not read it. Its zero digit is `-`, so encoded IDs can start with
    /// a hyphen.
    pub const BASE64_URL: Self =
        Self::new("-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz");

    /// Creates an alphabet from its characters, in digit order: the first
    /// character is zero.
    ///
    /// # Panics
    ///
    /// Panics unless `chars` holds between 2 and 94 distinct printable ASCII
    /// characters, without spaces. In a const context this is a compile-time
    /// error.
    pub const fn new(chars: &str) -> Self {
        let bytes = chars.as_bytes();
        assert!(bytes.len() >= 2, "an alphabet needs at least 2 characters");
        assert!(
            bytes.len() <= MAX_CHARS,
            "an alphabet holds at most 94 characters"
        );

        let mut alphabet = Self {
            chars: [0; MAX_CHARS],
            base: bytes.len() as u8,
            digits: [NO_DIGIT; 128],
        };
        let mut digit = 0;
        while digit < bytes.len() {
            let byte = bytes[digit];
            assert!(
                byte >= FIRST_CHAR && byte <= LAST_CHAR,
                "alphabet characters must be printable ASCII, without spaces"
            );
            assert!(
                alphabet.digits[byte as usize] == NO_DIGIT,
                "alphabet characters must not repeat"
            );
            alphabet.chars[digit] = byte;
            alphabet.digits[byte as usize] = digit as u8;
            digit += 1;
        }
        alphabet
    }

    /// The characters, in digit order.
    pub fn as_str(&self) -> &str {
        ascii_str(&self.chars[..usize::from(self.base)])
    }

    /// Number of characters needed to write any value up to `max`.
    #[doc(hidden)]
    pub const fn __encoded_len(&self, max: u64) -> usize {
        let base = self.base as u64;
        let mut len = 1;
        let mut rest = max / base;
        while rest > 0 {
            len += 1;
            rest /= base;
        }
        len
    }

    /// `base` is passed separately from `self` so a caller that knows it at
    /// compile time gets its divisions compiled as multiplications or shifts.
    #[inline]
    fn encode(&self, base: u64, len: usize, mut value: u64) -> Encoded {
        let mut bytes = [0; MAX_ENCODED_LEN];
        for byte in bytes[..len].iter_mut().rev() {
            *byte = self.chars[(value % base) as usize];
            value /= base;
        }
        debug_assert_eq!(value, 0);
        Encoded {
            bytes,
            len: len as u8,
        }
    }

    #[inline]
    fn decode(&self, base: u64, len: usize, text: &str) -> Result<u64, DecodeIdError> {
        if text.len() != len {
            return Err(DecodeIdError::Length {
                expected: len,
                found: text.len(),
            });
        }

        let mut value: u64 = 0;
        for (index, byte) in text.bytes().enumerate() {
            let digit = match self.digits.get(usize::from(byte)) {
                Some(&digit) if digit != NO_DIGIT => u64::from(digit),
                _ => return Err(DecodeIdError::Character { index }),
            };
            value = value
                .checked_mul(base)
                .and_then(|value| value.checked_add(digit))
                .ok_or(DecodeIdError::Overflow)?;
        }
        Ok(value)
    }
}

impl fmt::Debug for Alphabet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Alphabet").field(&self.as_str()).finish()
    }
}

/// Alphabets and encodings only ever hold ASCII bytes.
fn ascii_str(bytes: &[u8]) -> &str {
    debug_assert!(bytes.is_ascii());
    core::str::from_utf8(bytes).unwrap_or_default()
}

/// An ID type with an alphabet, declared with the `alphabet` option of
/// [`#[typedflake]`](macro@crate::typedflake).
///
/// Its methods are also generated as inherent methods, so the trait only needs
/// importing for code that is generic over ID types.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no alphabet",
    label = "this ID type cannot be encoded",
    note = "add `alphabet = ...` to its `#[typedflake(...)]` attribute"
)]
pub trait EncodedId: Id {
    /// Alphabet this ID type is written with.
    const ALPHABET: Alphabet;

    #[doc(hidden)]
    const __ENCODED_LEN: usize = Self::ALPHABET.__encoded_len(Self::__LAYOUT.raw_max);

    #[doc(hidden)]
    const __BASE: u64 = Self::ALPHABET.base as u64;

    /// Writes the ID in its alphabet, without allocating.
    ///
    /// Every ID of the type has the same length: shorter values are padded
    /// with the alphabet's first character.
    fn encode(self) -> Encoded {
        const { &Self::ALPHABET }.encode(Self::__BASE, Self::__ENCODED_LEN, self.get().__bits())
    }

    /// Reads an ID written by [`encode`](Self::encode).
    ///
    /// Only that exact form is accepted, so each ID has a single encoding.
    fn decode(text: &str) -> Result<Self, DecodeIdError> {
        let raw = const { &Self::ALPHABET }.decode(Self::__BASE, Self::__ENCODED_LEN, text)?;
        Ok(id::from_u64(raw)?)
    }
}

/// An ID written in its alphabet. Returned by `encode`.
///
/// It lives on the stack and reads as a `&str`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Encoded {
    bytes: [u8; MAX_ENCODED_LEN],
    len: u8,
}

impl Encoded {
    /// The encoded ID.
    pub fn as_str(&self) -> &str {
        ascii_str(&self.bytes[..usize::from(self.len)])
    }
}

impl Deref for Encoded {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Encoded {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Encoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Encoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// A string could not be decoded as an ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeIdError {
    /// The string does not have the length every encoded ID of this type has.
    Length {
        /// Length of an encoded ID, in bytes.
        expected: usize,
        /// Length of the string that was supplied, in bytes.
        found: usize,
    },
    /// The string has a character that is not in the alphabet.
    Character {
        /// Byte position of the character.
        index: usize,
    },
    /// The string encodes a number that does not fit in 64 bits.
    Overflow,
    /// The decoded integer is not a valid ID for the format.
    Invalid(InvalidId),
}

impl fmt::Display for DecodeIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Length { expected, found } => write!(
                f,
                "encoded ID has {found} characters; this ID type uses {expected}"
            ),
            Self::Character { index } => write!(
                f,
                "character at position {index} is not in the ID's alphabet"
            ),
            Self::Overflow => f.write_str("encoded ID does not fit in 64 bits"),
            Self::Invalid(_) => f.write_str("decoded integer is not a valid ID"),
        }
    }
}

impl std::error::Error for DecodeIdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(error) => Some(error),
            _ => None,
        }
    }
}

impl From<InvalidId> for DecodeIdError {
    fn from(error: InvalidId) -> Self {
        Self::Invalid(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_their_base_and_ascending_characters() {
        for (alphabet, base) in [
            (Alphabet::BASE36, 36),
            (Alphabet::BASE58, 58),
            (Alphabet::BASE62, 62),
            (Alphabet::BASE64_URL, 64),
        ] {
            assert_eq!(alphabet.as_str().len(), base);
            assert!(alphabet.as_str().as_bytes().is_sorted());
        }
    }

    #[test]
    fn encoded_length_covers_the_largest_value() {
        let binary = Alphabet::new("01");
        assert_eq!(binary.__encoded_len(0), 1);
        assert_eq!(binary.__encoded_len(1), 1);
        assert_eq!(binary.__encoded_len(2), 2);
        assert_eq!(binary.__encoded_len(u64::MAX), 64);

        assert_eq!(Alphabet::BASE62.__encoded_len(61), 1);
        assert_eq!(Alphabet::BASE62.__encoded_len(62), 2);
        assert_eq!(Alphabet::BASE62.__encoded_len(i64::MAX as u64), 11);
        assert_eq!(Alphabet::BASE62.__encoded_len(u64::MAX), 11);
        assert_eq!(Alphabet::BASE64_URL.__encoded_len(u64::MAX), 11);
        assert_eq!(Alphabet::BASE64_URL.__encoded_len((1 << 60) - 1), 10);
    }

    #[test]
    fn values_round_trip_at_the_edges_of_every_width() {
        let widest = Alphabet::new(
            "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
        );
        for alphabet in [Alphabet::new("01"), Alphabet::BASE62, widest] {
            for max in [1, 61, 62, 4095, i64::MAX as u64, u64::MAX] {
                let len = alphabet.__encoded_len(max);
                for value in [0, 1, max / 2, max - 1, max] {
                    let text = alphabet.encode(u64::from(alphabet.base), len, value);
                    assert_eq!(text.len(), len);
                    assert_eq!(
                        alphabet.decode(u64::from(alphabet.base), len, &text),
                        Ok(value)
                    );
                }
            }
        }
    }

    #[test]
    fn decoding_rejects_anything_encode_does_not_write() {
        let alphabet = Alphabet::BASE62;

        assert_eq!(
            alphabet.decode(62, 3, "0A"),
            Err(DecodeIdError::Length {
                expected: 3,
                found: 2
            })
        );
        assert_eq!(
            alphabet.decode(62, 3, "0-A"),
            Err(DecodeIdError::Character { index: 1 })
        );
        // Two bytes, so the length already matches a different string.
        assert_eq!(
            alphabet.decode(62, 3, "0é"),
            Err(DecodeIdError::Character { index: 1 })
        );
        assert_eq!(
            alphabet.decode(62, 11, "zzzzzzzzzzz"),
            Err(DecodeIdError::Overflow)
        );
    }

    #[test]
    #[should_panic(expected = "must not repeat")]
    fn repeated_characters_are_rejected() {
        let _ = Alphabet::new("abca");
    }

    #[test]
    #[should_panic(expected = "printable ASCII")]
    fn spaces_are_rejected() {
        let _ = Alphabet::new("a b");
    }

    #[test]
    #[should_panic(expected = "at least 2")]
    fn a_single_character_is_rejected() {
        let _ = Alphabet::new("a");
    }
}

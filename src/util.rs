//! Utilities

use bytes::{Buf, BufMut};

/// Mixin [get_chars](#tymethod.get_chars) into types implements [Buf](../../bytes/trait.Buf.html) and
/// [BufMut](../../bytes/trait.BufMut.html).
pub trait GetCharsMixin<B> {
    /// Iterate byte stream as UTF-8 string.
    ///
    /// This method returns a iterater over characters in the buffer.
    /// If read imcomplete character sequence, the sequence will be left in the buffer.
    ///
    /// ```
    ///     # use serialcat::util::{GetChars, GetCharsMixin as _};
    ///     # use bytes::{Buf as _, BufMut as _, BytesMut};
    ///
    ///     let mut buffer = BytesMut::new();
    ///     buffer.put_slice(&[
    ///         b'a',       // Alphabet a (1 byte character)
    ///         0x83,       // Invalid character (Do not appear in the first byte)
    ///         0xce, 0x93, // Greek Gamma (2 bytes character)
    ///         0xc7,       // Incomplete sequence (First byte of 2 bytes character)
    ///     ]);
    ///
    ///     let mut iter = buffer.get_chars();
    ///     assert_eq!(iter.next(), Some(GetChars::Char('a')));
    ///     assert_eq!(iter.next(), Some(GetChars::Err(0x83)));
    ///     assert_eq!(iter.next(), Some(GetChars::Char('Γ')));
    ///     assert_eq!(iter.next(), None); // 0xc7 was kept in the buffer
    ///
    ///     assert_eq!(buffer.get_u8(), 0xc7);
    /// ```
    fn get_chars(&mut self) -> GetCharsIter<B>;
}

impl<B> GetCharsMixin<B> for B
where
    B: Buf + BufMut,
{
    fn get_chars(&mut self) -> GetCharsIter<B> {
        GetCharsIter {
            inner: self,
            processing: [0; 4],
            processing_len: 0,
        }
    }
}

/// Interator to read chars.
///
/// see [GetCharsMixin::get_chars](trait.GetCharsMixin.html#tymethod.get_chars).
pub struct GetCharsIter<'a, B> {
    inner: &'a mut B,
    processing: [u8; 4],
    processing_len: usize,
}

/// Result of [GetCharsMixin::get_chars](trait.GetCharsMixin.html#tymethod.get_chars).
///
/// see [GetCharsMixin::get_chars](trait.GetCharsMixin.html#tymethod.get_chars).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GetChars {
    /// A valid character.
    Char(char),
    /// A invalid byte.
    Err(u8),
}

impl<'a, B> Iterator for GetCharsIter<'a, B>
where
    B: Buf + BufMut,
{
    type Item = GetChars;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = self.processing;
        let get_len = std::cmp::min(buf.len() - self.processing_len, self.inner.remaining());
        self.inner
            .copy_to_slice(&mut buf[self.processing_len..][..get_len]);
        let buf = &buf[..self.processing_len + get_len];

        if buf.is_empty() {
            return None;
        }

        let (str, err) = match std::str::from_utf8(buf) {
            Ok(str) => (str, None), // None will not be used
            Err(e) => {
                let str = unsafe { std::str::from_utf8_unchecked(&buf[..e.valid_up_to()]) };
                (str, e.error_len())
            }
        };

        if !str.is_empty() {
            let len = (1..)
                .find_map(|l| {
                    if str.is_char_boundary(l) {
                        Some(l)
                    } else {
                        None
                    }
                })
                .unwrap();
            self.processing[..buf.len() - len].copy_from_slice(&buf[len..]);
            self.processing_len = buf.len() - len;

            let c = str.chars().next().unwrap();
            Some(GetChars::Char(c))
        } else if err.is_some() {
            self.processing[..buf.len() - 1].copy_from_slice(&buf[1..]);
            self.processing_len = buf.len() - 1;

            Some(GetChars::Err(buf[0]))
        } else {
            self.inner.put_slice(buf);

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::BytesMut;

    #[test]
    fn get_chars() {
        let mut buffer = BytesMut::new();
        buffer.put_slice(&b"abcdef"[..]);
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('a')));
        assert_eq!(iter.next(), Some(GetChars::Char('b')));
        assert_eq!(iter.next(), Some(GetChars::Char('c')));
        assert_eq!(iter.next(), Some(GetChars::Char('d')));
        assert_eq!(iter.next(), Some(GetChars::Char('e')));
        assert_eq!(iter.next(), Some(GetChars::Char('f')));
        assert_eq!(iter.next(), None);
        assert!(!buffer.has_remaining());
    }

    #[test]
    fn get_chars_multibyte() {
        let mut buffer = BytesMut::new();
        buffer.put_slice("AΓあ🀄".as_bytes());
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('A'))); // 1 byte
        assert_eq!(iter.next(), Some(GetChars::Char('Γ'))); // 2 byte
        assert_eq!(iter.next(), Some(GetChars::Char('あ'))); // 3 byte
        assert_eq!(iter.next(), Some(GetChars::Char('🀄'))); // 4 byte
        assert_eq!(iter.next(), None);
        assert!(!buffer.has_remaining());
    }

    #[test]
    fn get_chars_invalid() {
        let mut buffer = BytesMut::new();
        buffer.put_slice(&b"a\x81\xc2\xe3\x84\xf5\x86\x87b"[..]);
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('a')));
        assert_eq!(iter.next(), Some(GetChars::Err(0x81))); // non-first character
        assert_eq!(iter.next(), Some(GetChars::Err(0xc2))); // 2 byte character without second byte
        assert_eq!(iter.next(), Some(GetChars::Err(0xe3))); // 3 byte character without last byte
        assert_eq!(iter.next(), Some(GetChars::Err(0x84))); //     second byte
        assert_eq!(iter.next(), Some(GetChars::Err(0xf5))); // 4 byte character without last byte
        assert_eq!(iter.next(), Some(GetChars::Err(0x86))); //     second byte
        assert_eq!(iter.next(), Some(GetChars::Err(0x87))); //     third byte
        assert_eq!(iter.next(), Some(GetChars::Char('b')));
        assert_eq!(iter.next(), None);
        assert!(!buffer.has_remaining());
    }

    #[test]
    fn get_chars_incomplete() {
        // 2 byte character `Γ` without last byte
        let mut buffer = BytesMut::new();
        buffer.put_slice(&"aΓ".as_bytes()[..2]);
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('a')));
        assert_eq!(iter.next(), None);
        assert_eq!(buffer.get_u8(), "Γ".as_bytes()[0]);
        assert!(!buffer.has_remaining());

        // 3 byte character `あ` without last byte
        let mut buffer = BytesMut::new();
        buffer.put_slice(&"aあ".as_bytes()[..3]);
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('a')));
        assert_eq!(iter.next(), None);
        assert_eq!(buffer.get_u8(), "あ".as_bytes()[0]);
        assert_eq!(buffer.get_u8(), "あ".as_bytes()[1]);
        assert!(!buffer.has_remaining());

        // 4 byte character `🀄` without last byte
        let mut buffer = BytesMut::new();
        buffer.put_slice(&"a🀄".as_bytes()[..4]);
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('a')));
        assert_eq!(iter.next(), None);
        assert_eq!(buffer.get_u8(), "🀄".as_bytes()[0]);
        assert_eq!(buffer.get_u8(), "🀄".as_bytes()[1]);
        assert_eq!(buffer.get_u8(), "🀄".as_bytes()[2]);
        assert!(!buffer.has_remaining());
    }

    #[test]
    fn get_chars_bytes_split() {
        // BytesMut may split a multibyte character to last and first in buffer

        let mut buffer = BytesMut::with_capacity(6);
        // internal buffer: b"______"
        buffer.put_slice(&b"abcde"[..]);
        // internal buffer: b"abcde_"
        buffer.advance(3);
        // internal buffer: b"___de_", start is 'd'
        buffer.put_slice("Γ".as_bytes());
        // internal buffer: b"\x93__de\xce", start is 'd', 'Γ' is "\xce\x93"
        let mut iter = buffer.get_chars();
        assert_eq!(iter.next(), Some(GetChars::Char('d')));
        assert_eq!(iter.next(), Some(GetChars::Char('e')));
        assert_eq!(iter.next(), Some(GetChars::Char('Γ')));
        assert_eq!(iter.next(), None);
        assert!(!buffer.has_remaining());
    }
}

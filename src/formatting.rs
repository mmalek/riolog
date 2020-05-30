use crate::result::Result;
use std::io::Write;

pub fn format_special_chars(
    buf: &[u8],
    writer: &mut impl Write,
    mut ctr_char_is_next: bool,
    eol: &[u8],
    after_eol: &[u8],
) -> Result<bool> {
    let mut last_slice_is_empty = false;

    for s in buf.split(|&c| c == b'\\') {
        if s.is_empty() {
            if ctr_char_is_next {
                last_slice_is_empty = true;
                ctr_char_is_next = false;
            } else {
                writer.write_all(b"\\")?;
                last_slice_is_empty = false;
                ctr_char_is_next = true;
            }
        } else if ctr_char_is_next {
            match s[0] {
                b'0' => writer.write_all(b"\0")?,
                b'n' => {
                    writer.write_all(eol)?;
                    writer.write_all(after_eol)?;
                }
                b'r' => {}
                b't' => writer.write_all(b"\t")?,
                b'?' => writer.write_all(b"?")?,
                b'\'' => writer.write_all(b"\'")?,
                b'\"' => writer.write_all(b"\"")?,
                _ => {
                    writer.write_all(b"\\")?;
                    writer.write_all(&s[0..1])?;
                }
            }
            writer.write_all(&s[1..])?;
            last_slice_is_empty = false;
            ctr_char_is_next = true;
        } else {
            if last_slice_is_empty {
                writer.write_all(b"\\")?;
            }
            writer.write_all(s)?;
            last_slice_is_empty = false;
            ctr_char_is_next = true;
        }
    }

    Ok(last_slice_is_empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_special_chars_plain() -> Result<()> {
        let in_buf = b"abcdefg";
        let mut out_buf = Vec::<u8>::new();
        let last_slice_is_empty = format_special_chars(in_buf, &mut out_buf, false, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, false);
        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn format_special_chars_special_chars() -> Result<()> {
        let in_buf = b"a\\nb\\tc\\\'d\\\"e\\\\fg";
        let mut out_buf = Vec::<u8>::new();
        let last_slice_is_empty = format_special_chars(in_buf, &mut out_buf, false, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, false);
        assert_eq!(out_buf, b"a\nb\tc\'d\"e\\fg");

        Ok(())
    }

    #[test]
    fn format_special_chars_unknown_special_chars() -> Result<()> {
        let in_buf = b"a\\ab\\bc\\cd";
        let mut out_buf = Vec::<u8>::new();
        let last_slice_is_empty = format_special_chars(in_buf, &mut out_buf, false, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, false);
        assert_eq!(out_buf, in_buf);

        Ok(())
    }

    #[test]
    fn format_special_chars_slash_eol() -> Result<()> {
        let in_buf = b"abc\\";
        let mut out_buf = Vec::<u8>::new();
        let last_slice_is_empty = format_special_chars(in_buf, &mut out_buf, false, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, true);
        assert_eq!(out_buf, b"abc");

        Ok(())
    }

    #[test]
    fn format_special_chars_ctr_char_is_next() -> Result<()> {
        let in_buf = b"nabc";
        let mut out_buf = Vec::<u8>::new();
        let ctr_char_is_next = true;
        let last_slice_is_empty =
            format_special_chars(in_buf, &mut out_buf, ctr_char_is_next, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, false);
        assert_eq!(out_buf, b"\nabc");

        Ok(())
    }

    #[test]
    fn format_special_chars_ctr_char_is_next_slash() -> Result<()> {
        let in_buf = b"\\abc";
        let mut out_buf = Vec::<u8>::new();
        let ctr_char_is_next = true;
        let last_slice_is_empty =
            format_special_chars(in_buf, &mut out_buf, ctr_char_is_next, b"\n", b"")?;

        assert_eq!(last_slice_is_empty, false);
        assert_eq!(out_buf, b"\\abc");

        Ok(())
    }
}

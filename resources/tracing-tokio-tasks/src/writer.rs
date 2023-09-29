use std::{
    io::{self, ErrorKind},
    str,
};

pub struct HtmlWriter<T>
where
    T: io::Write,
{
    inner: T,
}

impl<T> HtmlWriter<T>
where
    T: io::Write,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> io::Write for HtmlWriter<T>
where
    T: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let escaped = convert_to_html(buf).map(|s| s.into_bytes());
        let buf = escaped.as_deref().unwrap_or(buf);

        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut buf = buf;
        let escaped = convert_to_html(buf).map(|s| s.into_bytes());
        if let Some(escaped) = escaped.as_deref() {
            buf = escaped;
        }

        while !buf.is_empty() {
            match self.inner.write(buf) {
                Ok(0) => {
                    return Err(io::Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn convert_to_html(buf: &[u8]) -> Option<String> {
    let utf8 = str::from_utf8(buf).ok()?;
    ansi_to_html::convert(utf8, true, true).ok()
}

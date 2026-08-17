use std::io::{self, Write};

/// A writer that fails every write the way a closed pipe does, as `jute
/// self.list | head -1` leaves stdout once `head` has exited.
#[derive(Debug)]
pub struct BrokenPipeWriter;

impl Write for BrokenPipeWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

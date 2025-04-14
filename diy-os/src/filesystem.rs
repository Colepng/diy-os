pub mod ustar;

pub trait File {
    fn read(&self) -> Result<&[u8], INError>;

    fn write(&mut self, buf: impl Into<&[u8]>) -> Result<usize, OUTError>;
}

#[derive(Debug)]
pub enum OUTError {
    WriteLargerThenMaxFileSize,
    NotWritable,
}

#[derive(Debug)]
pub enum INError {
    NotReadable,
}

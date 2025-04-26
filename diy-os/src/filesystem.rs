pub mod ustar;

pub trait File {
    /// Reads the file
    ///
    /// # Errors
    ///
    /// This function will return an error if the file can't be read.
    fn read(&self) -> Result<&[u8], INError>;

    /// Write to the file
    ///
    /// # Errors
    ///
    /// This function will return an error if the file can't be written too.
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

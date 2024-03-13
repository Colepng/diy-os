pub mod ustar;

pub trait File<WRITABLE: Writeability, READABLE: Readabitly> {
    fn read(&self) -> Result<&[u8], INError>;

    fn write(&mut self, buf: impl Into<&[u8]>) -> Result<usize, OUTError>;

    fn into_typesate_file(self) -> TypestateFile<WRITABLE, READABLE, Self>;
}

/// Represents that a given implementer is a valid state of writeability
pub trait Writeability {}

pub struct Writable;
impl Writeability for Writable {}
pub struct Unwritable;
impl Writeability for Unwritable {}

/// Represents that a given implementer is a valid state of readability
pub trait Readabitly {}
pub struct Readable;
impl Readabitly for Readable {}
pub struct Unreadable;
impl Readabitly for Unreadable {}

#[derive(Debug)]
pub enum OUTError {
    WriteLargerThenMaxFileSize,
    NotWritable,
}

#[derive(Debug)]
pub enum INError {
    NotReadable,
}

pub struct TypestateFile<WRITABLE, READABLE, FILE>
where
    WRITABLE: Writeability,
    READABLE: Readabitly,
    FILE: File<WRITABLE, READABLE> + ?Sized,
{
    writable: WRITABLE,
    readable: READABLE,
    inner: FILE,
}

impl<WRITABLE: Writeability, READABLE: Readabitly, FILE: File<WRITABLE, READABLE>>
    TypestateFile<WRITABLE, READABLE, FILE>
{
    pub fn into_file(self) -> FILE {
        self.inner
    }
}

impl<WRITABLE: Writeability, FILE: File<WRITABLE, Readable>> TypestateFile<WRITABLE, Readable, FILE> {
    pub fn read(&self) -> Result<&[u8], INError> {
        self.inner.read()
    }
}

impl<READABLE: Readabitly, FILE: File<Writable, READABLE>> TypestateFile<Writable, READABLE, FILE> {
    pub fn write(&mut self, buf: impl Into<&[u8]>) -> Result<usize, OUTError>{
        self.inner.write(buf)
    }
}

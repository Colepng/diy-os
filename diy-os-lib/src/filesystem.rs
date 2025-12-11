use alloc::{string::String, vec::Vec};

pub mod gpt;
pub mod ustar;

// pub trait File {
//     /// Reads the file
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if the file can't be read.
//     fn read(&self) -> Result<&[u8], INError>;
//
//     /// Write to the file
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if the file can't be written too.
//     fn write(&mut self, buf: impl Into<&[u8]>) -> Result<usize, OUTError>;
// }

#[derive(Debug)]
pub enum OUTError {
    WriteLargerThenMaxFileSize,
    NotWritable,
}

#[derive(Debug)]
pub enum INError {
    NotReadable,
}

// pub trait Filesystem {}
//
// pub struct Drives<'a> {
//     drives: Vec<&'a mut dyn Filesystem>,
// }

#[derive(Debug)]
pub struct File<'a> {
    pub name: String,
    pub data: &'a [u8],
}

pub struct Dir<'a> {
    pub name: String,
    pub dirs: Vec<Self>,
    pub files: Vec<File<'a>>,
}

pub struct Root<'a> {
    pub dirs: Vec<Self>,
    pub files: Vec<File<'a>>,
}

// impl Root<'_> {
//     pub fn open(path: &str) -> File {}
// }

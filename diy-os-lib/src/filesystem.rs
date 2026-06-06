use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};

use crate::device_manager::BlockDeviceError;
use crate::filesystem::gpt::PartionTableHeaderError;

pub mod gpt;
pub mod ustar;

#[derive(thiserror::Error, Debug)]
pub enum FileSystemSetupError {
    #[error("The underlaying block device experienced an error")]
    BlockDeviceError(#[from] BlockDeviceError),
    #[error("Encountered an error while parsing the gpt header, `{0}`")]
    PartionTableHeaderError(#[from] PartionTableHeaderError),
}

pub struct VFS {
    mount_point: Box<dyn FileSystem>,
}

impl VFS {
    pub fn new(mount_point: Box<dyn FileSystem>) -> Self {
        Self { mount_point }
    }

    pub fn open(&mut self, path: &str) -> Option<Box<dyn FileTrait + '_>> {
        self.mount_point.open(path)
    }
}

pub trait FileSystem {
    fn open(&mut self, path: &str) -> Option<Box<dyn FileTrait + '_>>;
}

// pub trait DirTrait {
//     // fn
// }

pub trait FileTrait {
    /// Reads the file
    ///
    /// # Errors
    ///
    /// This function will return an error if the file can't be read.
    fn read(&self, buf: &mut [u8]) -> Result<usize, INError>;

    /// Write to the file
    ///
    /// # Errors
    ///
    /// This function will return an error if the file can't be written too.
    fn write(&mut self, buf: &[u8]) -> Result<usize, OUTError>;
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

// pub trait Filesystem {}
//
// pub struct Drives<'a> {
//     drives: Vec<&'a mut dyn Filesystem>,
// }

#[derive(Debug)]
pub struct File {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct Dir {
    pub name: String,
    pub dirs: Vec<Self>,
    pub files: Vec<File>,
}

pub struct Root {
    pub dirs: Vec<Self>,
    pub files: Vec<File>,
}

// impl Root<'_> {
//     pub fn open(path: &str) -> File {}
// }

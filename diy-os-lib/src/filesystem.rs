use alloc::sync::Arc;

use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};

use crate::device_manager::{BlockDevice, BlockDeviceError};
use crate::filesystem::gpt::PartionTableHeader;
use crate::multitasking::mutex::Mutex;

pub mod gpt;
pub mod ustar;

#[derive(thiserror::Error, Debug)]
pub enum FileSystemSetupError {
    #[error("The underlaying block device experienced an error")]
    BlockDeviceError(#[from] BlockDeviceError),
}

// fn setup_filesystem(
//     device: Arc<Mutex<dyn BlockDevice>>,
// ) -> Result<Box<dyn FileSystem>, FileSystemSetupError> {
//     let mut sector_buffer = [0u8; 92];
//
//     let mut drive = device.acquire();
//
//     drive.read_sectors(1, 1, &mut sector_buffer)?;
//
//     // let sector_buffer: [u8; 92] = array::from_fn(|x| sector_buffer[x]);
//
//     let header = unsafe { core::mem::transmute::<[u8; 92], PartionTableHeader>(sector_buffer) };
//
//     println!("header: {header:#?}");
//     //
//     // // assert!(header.validate(addr));
//     //
//     // assert!(128 == header.size_of_partion_entry);
//
//     let mut partion_entry = [0u8; 128];
//
//     drive.read_sectors(2, 1, &mut partion_entry)?;
//
//     let partion = unsafe { core::mem::transmute::<[u8; 128], PartitionEntry>(partion_entry) };
//
//     // Has not been called yet
//     // unsafe { helper.init(addr.try_into()?, partion.starting_lba.try_into()?) };
//
//     let name = partion.name().unwrap();
//
//     println!("partion {name}, partion {:?}", partion);
//     // println!("fs: {:X}", partion.partion_type_guid);
//     // let fs = partion.get_fs().unwrap();
//     // println!("fs {fs:?}");
//     //
//     // let partion_addr = helper.addr_from_partion_lba(0);
//     let mut bios = [0u8; 36];
//
//     drive.read_sectors(partion.starting_lba, 1, &mut bios)?;
//
//     let bios = unsafe { core::mem::transmute::<[u8; 36], BIOSParameterBlock>(bios) };
//
//     println!("bpb : {bios:?}");
//
//     let fat_type = bios.get_fat_type();
//
//     println!("fat type {:?}", fat_type);
// }

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

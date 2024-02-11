use core::{ffi::{c_char, c_uchar}, fmt::write};

use alloc::{slice, vec::Vec};

pub struct Ustar {
    ptr: *mut MetaData,
}

impl Ustar {
    const BLOCK_SIZE: u16 = 512;

    /// Crates a new Ustar ramdisk
    ///
    /// # Safety
    /// Caller must make that the addr is valid
    pub const unsafe fn new(addr: u64) -> Self {
        Self {
            ptr: addr as *mut MetaData,
        }
    }

    pub fn get_files(&self) -> Vec<File> {
        let mut files: Vec<File> = Vec::new();

        let mut ptr = self.ptr;
        loop {
            // if the next 2 block are empty exit
            if unsafe { slice::from_raw_parts(ptr as *const u8, 1024) }
                .iter()
                .all(|x| *x == 0)
            {
                break;
            }

            let metadata = unsafe { &mut *ptr };

            let has_data = metadata.oct_to_bin() != 0;

            let mut file = File {
                inode: metadata,
                data: None,
            };

            if has_data {
                let data = unsafe { &mut *(ptr.offset(1) as *mut Data) };

                file.data = Some(data);

                ptr = unsafe { ptr.offset(2) };
            } else {
                ptr = unsafe { ptr.offset(1) };
            }

            files.push(file);
        }

        return files;
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct MetaData {
    pub file_name: [c_char; 100],
    file_mode: u64,
    uid: u64,
    gui: u64,
    pub file_size: [c_uchar; 12],
    last_modfication_time: [u8; 12],
    check_sum: u64,
    type_flag: u8,
    pub link_name: [c_char; 100],
    pub ustar_indicator: [c_char; 6],
    pub ustar_version: [c_char; 2],
    pub user_name: [c_char; 32],
    pub group_name: [c_char; 32],
    pub device_major_number: [c_char; 8],
    pub device_minor_number: [c_char; 8],
    pub file_name_prefix: [c_char; 155],
    _padding: [u8; 12],
}

impl MetaData {
    pub fn oct_to_bin(&self) -> u32 {
        let mut result: u32 = 0;

        for i in self.file_size {
            if i == 0 {
                break;
            }

            result *= 8;
            result += (i - b'0') as u32;
        }

        return result;
    }
}

pub struct Data {
    pub bytes: [u8; 512],
}

#[repr(u8)]
pub enum FileType {
    NormalFile = b'0',
    HardLink = b'1',
    SymbolicLink = b'2',
    CharDevice = b'3',
    BlockDevice = b'4',
    Directory = b'5',
    NamedPipe = b'6',
}

pub struct File {
    inode: &'static mut MetaData,
    data: Option<&'static mut Data>,
}

impl File {
    pub fn get_raw_bytes(&self) -> Option<&[u8; 512]> {
        if let Some(data) = &self.data {
            Some(&data.bytes)
        } else {
            return None;
        }
    }
}

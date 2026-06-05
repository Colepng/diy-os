use crate::fat::{Cluster, Directory, EntryFlags, LongFileName, Sector, UnknownEntry};
use core::str;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use diy_os::device_manager::BlockDevice;
use diy_os::filesystem::{FileSystem, FileTrait};
use diy_os::multitasking::mutex::Mutex;
use either::Either::{Left, Right};

use crate::fat::fat16::ExtenedBootRecord as Fat16EBR;
use crate::fat::fat32::ExtenedBootRecord as Fat32EBR;
use crate::println;

extern crate alloc;

pub fn primitive_memmapped_fat32_read_only_driver(
    partion_addr: u64,
    boot_recorded_addr: usize,
) -> anyhow::Result<!> {
    let fat32_table = Fat32EBR::from_addr(boot_recorded_addr);

    assert!(fat32_table.valid_signature());

    println!("ebr 32: {fat32_table:?}");

    // let sectors =
    // get_entire_slice_from_cluster(fat32_table.cluster_of_root_dir, bios, partion_addr);

    let cluster = fat32_table.cluster_of_root_dir;

    println!("root cluster: {cluster:?}");

    let sector = cluster.first_sector_of_cluster_fat32(&fat32_table.bpb, fat32_table);

    println!("sector: {sector}");

    let addr = partion_addr + u64::from(sector * 512);

    // [u32; 128] is equal to a sector size, 512 bytes
    let ptr: *const UnknownEntry = core::ptr::without_provenance(usize::try_from(addr).unwrap());

    let sector = unsafe { ptr.read() };

    println!("{:?}", sector);

    // for entry in sector.0.iter() {
    //     println!("{}", entry.empty());
    // }

    // let table_value = get_table_value(fat32_table.cluster_of_root_dir, bios, addr);
    //
    // if table_value >= 0x0FFF_FFF8 {
    //     println!("out of clusters in chain");
    // } else if table_value == 0x0FFF_FFF7 {
    //     println!("bad cluster");
    // } else {
    //     println!("next cluster {table_value:X}");
    // }

    // for sector in sectors {
    //     for entry in sector.0.iter() {
    //         if !entry.empty() {
    //             println!("entry: {:#?}", entry);
    //         }
    //     }
    // }

    // SAFETY: Scheduler is not held
    // unsafe { exit() };
    todo!()
}

fn read_directory(dir: Directory, drive: &mut dyn BlockDevice, partion_lba: u64, ebr: &Fat16EBR) {
    let cluster = dir.cluster();
    let sector = cluster.first_sector_of_cluster(&ebr.bpb);

    let mut entry = [0u8; 512];

    drive
        .read_sectors(u64::from(sector) + partion_lba, 1, &mut entry)
        .unwrap();

    let entry = unsafe { core::mem::transmute::<[u8; 512], [UnknownEntry; 16]>(entry) };

    let entry = entry[0].get_entry();

    println!("dir entry {entry:?}");
}

struct Fat16FS {
    partion_lba: u64,
    ebr: Fat16EBR,
    drive: Arc<Mutex<dyn BlockDevice>>,
}

impl Fat16FS {
    fn open_dir(&mut self, path: &str) -> Option<Vec<File>> {
        println!("got path: {path}");
        // if we are at the root dir
        let sector: u64 = if path == "/" {
            println!("root dir");
            u64::from(self.ebr.bpb.first_date_sector())
                - u64::from(self.ebr.bpb.get_size_of_root_dir())
        } else {
            println!("path: {path}");
            let slash_index = path.rfind('/').unwrap();

            let dir_path = &path[..=slash_index];

            println!("open dir, with a passed in {path}");
            let dir = self.open_dir(dir_path).unwrap();
            println!("opened dir, with a passed in {path}");

            let mut sector = None;

            for file in dir {
                let path_1 = file.name; //alloc::format!("{}{}", dir_path, file.name);
                let path_2 = &path[(slash_index + 1)..];
                println!("path1: {path_1}");
                println!("path2: {path_2}");
                if path_1 == path_2 {
                    if file.metadata.flags.intersects(EntryFlags::Directory) {
                        sector = Some(
                            file.metadata
                                .cluster()
                                .first_sector_of_cluster(&self.ebr.bpb),
                        );
                        break;
                    }

                    return None;
                }
            }

            u64::from(sector?)
        };

        let mut entry = [0u8; 512];

        self.drive
            .acquire()
            .read_sectors(sector + self.partion_lba, 1, &mut entry)
            .unwrap();

        let entry = unsafe { core::mem::transmute::<[u8; 512], [UnknownEntry; 16]>(entry) };

        let mut files: Vec<File> = Vec::new();
        let mut long_file_entries: Vec<LongFileName> = Vec::new();

        for entry in entry {
            // no more entries in cluster
            if entry.empty() {
                break;
            }

            if entry.unused() {
                long_file_entries.clear();
                continue;
            }

            match entry.get_entry() {
                Some(Right(long_file_name)) => long_file_entries.push(long_file_name),
                Some(Left(dir)) => {
                    let name: String = long_file_entries
                        .iter()
                        .map(LongFileName::name_as_str)
                        .collect();

                    long_file_entries.clear();

                    //TODO: smth is wrong here

                    // let mut dir_name = dir.name_as_str();
                    //
                    // dir_name.push_str(&name);
                    //
                    // let temp = Vec::new();

                    files.push(File {
                        name,
                        metadata: dir,
                    });
                }
                None => todo!(),
            }
        }

        Some(files)
    }
}

// impl Fat16File {
//     fn get_files_in_dir(&mut self, path: &str) {
//         let slash_index = path.rfind('/').unwrap();
//     }
// }

impl FileSystem for Fat16FS {
    fn open(&mut self, path: &str) -> Option<Box<dyn diy_os::filesystem::FileTrait + '_>> {
        let slash_index = path.rfind('/').unwrap();

        let dir_path = &path[..slash_index];

        let dir = self.open_dir(dir_path).unwrap();

        let mut file_entry = None;

        println!("Looking for file");
        for file in dir {
            let path_1 = &file.name;
            let path_2 = &path[(slash_index + 1)..];
            println!("file: {file:#?}");
            println!("path1: {path_1}");
            println!("path2: {path_2}");
            if path_1 == path_2 {
                if file.metadata.flags.intersects(EntryFlags::Directory) {
                    return None;
                }

                file_entry = Some(file);
                break;
            }
        }
        println!("found: {file_entry:?}");

        Some(Box::new(Fat16File {
            drive: self.drive.clone(),
            metadata: file_entry?.metadata,
            ebr: &self.ebr,
            partion_lba: self.partion_lba,
        }))
    }
}

#[derive(Debug)]
struct File {
    name: String,
    metadata: Directory,
}

struct Fat16File<'a> {
    drive: Arc<Mutex<dyn BlockDevice>>,
    metadata: Directory,
    ebr: &'a Fat16EBR,
    partion_lba: u64,
}

impl FileTrait for Fat16File<'_> {
    fn read(&self, buf: &mut [u8]) -> Result<usize, diy_os::filesystem::INError> {
        let sector = self
            .metadata
            .cluster()
            .first_sector_of_cluster(&self.ebr.bpb);

        let () = self
            .drive
            .acquire()
            .read_sectors(u64::from(sector) + self.partion_lba, 1, buf)
            .unwrap();

        Ok(512)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, diy_os::filesystem::OUTError> {
        todo!()
    }
}

//TODO remove function and inline
pub fn fat16_read_only(
    partion_lba: u64,
    device: Arc<Mutex<dyn BlockDevice>>,
) -> Box<dyn FileSystem> {
    let ebr = Fat16EBR::new(&device, partion_lba);

    Box::new(Fat16FS {
        partion_lba,
        ebr,
        drive: device,
    })
}

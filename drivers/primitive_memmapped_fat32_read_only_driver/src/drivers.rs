use core::ascii::Char;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::str;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use diy_os::filesystem::File;
use diy_os::multitasking::{exit, sleep};
use diy_os::timer::Miliseconds;
use either::Either;

use crate::fat::fat16::ExtenedBootRecord as Fat16EBR;
use crate::fat::fat32::ExtenedBootRecord as Fat32EBR;
use crate::fat::{BIOSParameterBlock, Directory, LongFileName, Sector, UnknownEntry};
use crate::{Helper, println};

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

#[allow(dead_code)]
pub fn primitive_memmapped_fat16_read_only_driver(partion_addr: usize, helper: &Helper) {
    /// this iterator is terribly unsafe, it w
    struct MaybeUninitIterator<T, const N: usize> {
        ptr: *const MaybeUninit<T>,
        count: usize,
    }

    impl<T, const N: usize> MaybeUninitIterator<T, N> {
        pub const fn new(ptr: *const MaybeUninit<T>) -> Self {
            Self { ptr, count: 0 }
        }
    }

    impl<T, const N: usize> Iterator for MaybeUninitIterator<T, N> {
        type Item = MaybeUninit<T>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.count < N {
                let item = unsafe { self.ptr.read_unaligned() };
                self.ptr = unsafe { self.ptr.add(1) };
                Some(item)
            } else {
                None
            }
        }
    }

    let ebr = Fat16EBR::from_addr(partion_addr);

    assert!(ebr.valid_signature());

    let sector = ebr.bpb.first_date_sector() - ebr.bpb.get_size_of_root_dir();

    println!("sector: {sector}");

    let root_dir_addr = helper.addr_from_partion_lba(sector.into());

    let entry_ptr = core::ptr::with_exposed_provenance::<MaybeUninit<UnknownEntry>>(root_dir_addr);

    let iter = MaybeUninitIterator::<UnknownEntry, 4>::new(entry_ptr);

    iter.take_while(|entry| unsafe { entry.as_bytes()[0].assume_init() } != 0)
        .map(|entry| unsafe { entry.assume_init() })
        .filter_map(|entry| {
            if entry.unused() {
                None
            } else {
                entry.get_entry()
            }
        })
        .filter_map(Either::left)
        // .for_each(|entry| println!("{entry:#?}"));
        .filter(|entry| entry.extension == [Char::CapitalT, Char::CapitalX, Char::CapitalT])
        .map(|text_file| {
            let cluster = text_file.cluster();
            println!("cluster: {cluster:?}");

            let sector = cluster.first_sector_of_cluster(&ebr.bpb);
            println!("sector: {sector:?}");

            let sector_addr = helper.addr_from_partion_lba(sector.try_into().unwrap());
            println!("sector addr: {sector_addr:#?}");

            let sector_ptr = core::ptr::with_exposed_provenance::<u8>(sector_addr);
            let len = text_file.size_in_bytes;

            println!("sector ptr: {sector_ptr:?}");

            let slice_ptr = core::ptr::slice_from_raw_parts(sector_ptr, len.try_into().unwrap());
            let slice = unsafe { slice_ptr.as_ref().unwrap() };
            // println!("slice: {slice:#?}");

            File {
                name: text_file.name_as_str(),
                data: slice,
            }
        })
        .for_each(|file| {
            println!("name: {}", file.name);
            let text = str::from_utf8(file.data).unwrap();
            println!("text: {text}");
        });
}

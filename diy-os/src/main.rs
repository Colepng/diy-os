#![no_std]
#![no_main]
#![feature(optimize_attribute)]
#![feature(custom_test_frameworks)]
#![feature(exposed_provenance)]
#![test_runner(diy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![warn(clippy::pedantic, clippy::nursery, clippy::perf, clippy::style)]
#![deny(
    clippy::suspicious,
    clippy::correctness,
    clippy::complexity,
    clippy::missing_const_for_fn,
    unsafe_op_in_unsafe_fn
)]

extern crate alloc;

use bootloader_api::{
    config::{Mapping, Mappings},
    entry_point, BootInfo, BootloaderConfig,
};
use core::panic::PanicInfo;
use diy_os::{
    acpi::system_description_tables::{RSDP, XSDT},
    allocator, hlt_loop, init,
    memory::BootInfoFrameAllocator,
    println,
};

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    let mut mappings = Mappings::new_default();
    mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings = mappings;

    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
extern "Rust" fn main(boot_info: &'static mut BootInfo) -> ! {
    let offset = boot_info.physical_memory_offset.into_option().unwrap();
    let xsdp_physical_address = boot_info.rsdp_addr.into_option().unwrap();

    init(boot_info);

    println!("Hello, world!");

    let xsdp_virtual_address = xsdp_physical_address + offset;

    let xsdp_ptr: *mut RSDP = core::ptr::from_exposed_addr_mut(xsdp_virtual_address as usize);

    let xsdp = unsafe { &mut *xsdp_ptr };

    println!("{:#?}", xsdp);

    let xsdt_virtual_address = xsdp.xsdt_address + offset;

    let xsdt = unsafe { XSDT::new_ref(xsdp.xsdt_address as usize, offset as usize) };

    let tables = xsdt.to_slice_of_tables(offset as usize);

    for table in tables {
        println!(
            "signature: {}",
            core::str::from_utf8(&table.signature).unwrap()
        )
    }

    // let xdst = XDST::new(xsdt_virtual_address as usize);
    //
    // println!("header {:#?}", xdst.xdst.sdt);
    //
    // let table_ptr = unsafe { (xdst.xdst as *mut XDST).add(1) };
    //
    // let table_count = xdst.xdst.sdt.num_sdt_entries();
    //
    // let table_phy_ptrs = unsafe {
    //     core::slice::from_raw_parts_mut(table_ptr.cast::<*mut ACPISDTHeader>(), table_count)
    // };
    //
    // for table_phy_ptr in table_phy_ptrs {
    //     let virt_ptr = unsafe { table_phy_ptr.byte_add(offset as usize) };
    //     let table = unsafe { &mut *virt_ptr };
    //     println!("sign: {}", core::str::from_utf8(&table.signature).unwrap());
    // }

    // for i in 0.. {
    //     let acpi_ptr: *const ACPISDTHeader = unsafe { core::ptr::from_exposed_addr((xdst.xdst as ) };
    //     let acpi_header = unsafe { & *acpi_ptr };
    //     println!("sign: {:?}", core::str::from_utf8(&acpi_header.signature));
    // }

    // let table_ptr: *const ACPISDTHeader = core::ptr::from_exposed_addr(pointers[0] as usize);
    //
    // let table = unsafe { & *table_ptr };
    //
    // println!("first table {:#?}", table);

    // let ptrs_virt_start = unsafe { rdsp_ptr.add(1).cast::<u8>() };
    // let ptrs_bytes_len = rdsp.length - mem::size_of::<ACPISDTHeader>() as u32;
    //
    // let ptrs_bytes = unsafe { core::slice::from_raw_parts(ptrs_virt_start, ptrs_bytes_len as usize) } ;
    //
    // let table_iter = ptrs_bytes.chunks(8).map(|ptr_bytes_src| {
    //     let mut ptr_dst = [0; mem::size_of::<usize>()];
    //     let common_ptr_size = usize::min(mem::size_of::<usize>(), ptr_bytes_src.len());
    //
    //     ptr_dst[..common_ptr_size].copy_from_slice(&ptr_bytes_src[..common_ptr_size]);
    //
    //     usize::from_le_bytes(ptr_dst) as *const ACPISDTHeader
    // });
    //
    // for table_ptr in table_iter.take(1) {
    //     let table = unsafe { & *table_ptr };
    //
    //     println!("signature: {}", core::str::from_utf8(&table.signature).unwrap());
    // }

    // println!("len {:#?}", ptrs_bytes_len);

    hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    diy_os::test_panic_handler(info)
}

// test to make sure tests won't panic
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

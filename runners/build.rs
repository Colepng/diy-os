use bootloader::DiskImageBuilder;
use std::{env, path::PathBuf};

fn main() {
    std::env::set_current_dir("../").unwrap();

    // set by cargo for the kernel artifact dependency
    let kernel_path = env::var("CARGO_BIN_FILE_KERNEL").unwrap();
    let mut disk_builder = DiskImageBuilder::new(PathBuf::from(kernel_path));

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let uefi_path = out_dir.join("os-uefi.img");

    disk_builder.set_ramdisk("sfs.img".into());

    // create the disk images
    disk_builder.create_uefi_image(&uefi_path).unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
}

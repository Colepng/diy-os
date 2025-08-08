use std::{env, fs};

fn main() {
    let current_exe = env::current_exe().unwrap();
    let uefi_target = current_exe.with_file_name("uefi.img");

    fs::copy(env!("UEFI_IMAGE"), &uefi_target).unwrap();

    println!("UEFI disk image at {}", uefi_target.display());
}

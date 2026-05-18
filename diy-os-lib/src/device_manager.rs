use crate::println;
use alloc::sync::Arc;

use alloc::boxed::Box;
use anyhow::Error;

use crate::multitasking::mutex::Mutex;
use crate::pci;
use crate::pci::ide::create_ide_controller;
use crate::pci::{ClassCode, MassStorageSubclass};
use alloc::vec::Vec;

pub struct DeviceManager {
    devices: Vec<Box<dyn Device>>,
}

impl DeviceManager {
    pub fn print_devices(&self) {
        for device in &self.devices {
            device.print_device();
        }
    }
}

const PCI_AVAILABLE: bool = true;

/// # Errors
/// Will error if ide controller fails to initialize
pub fn init_device_manager() -> Result<DeviceManager, Error> {
    let mut devices: Vec<Box<dyn Device>> = Vec::new();
    // check what buses/protocols supported, rn only pci, assuming it's available
    const {
        assert!(PCI_AVAILABLE);
    }

    let pci = pci::enumerate();

    if let Some(device) = pci.iter().find(|device| {
        device.class_code == ClassCode::MassStorageController
            && device.subclass == MassStorageSubclass::Ide
    }) {
        // assuming ide, I am lazyyy
        devices.push(Box::new(create_ide_controller(*device)?));
    }

    Ok(DeviceManager { devices })
}

type DeviceWrapper = Arc<Mutex<dyn Device>>;

pub trait Device: core::fmt::Debug + Send + Sync {
    fn children(&self) -> Option<Box<dyn Iterator<Item = DeviceWrapper>>>;

    fn print_device(&self) {
        println!("device: {self:#?}");

        if let Some(children) = self.children() {
            for device in children {
                #[allow(clippy::redundant_closure_for_method_calls)] // false positive submit pr
                device.with_ref(|d| d.print_device());
            }
        }
    }
}

// TODO: proper errors
pub trait BlockDevice: Device {
    /// # Errors
    fn read_sectors(&mut self, lba: u64, count: u8, buffer: &mut [u8]) -> Result<(), Error>;
    /// # Errors
    fn write_sectors(&mut self, lba: u64, count: u8, buffer: &[u8]) -> Result<(), Error>;
    fn total_sectors(&self) -> u64;
    fn sector_size(&self) -> usize {
        512
    }
}

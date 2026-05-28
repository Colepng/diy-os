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
    pub devices: Vec<Arc<Mutex<dyn Device>>>,

    pub block_devices: Vec<Arc<Mutex<dyn BlockDevice>>>,
}

impl DeviceManager {
    pub fn print_devices(&self) {
        for device in &self.devices {
            device.acquire().print_device();
        }
    }

    pub fn register_device(&mut self, device: Arc<Mutex<dyn Device>>) {
        self.devices.push(device);
    }

    pub fn register_block_device(&mut self, device: Arc<Mutex<dyn BlockDevice>>) {
        self.block_devices.push(device);
    }
}

const PCI_AVAILABLE: bool = true;

/// # Errors
/// Will error if ide controller fails to initialize
pub fn init_device_manager() -> Result<DeviceManager, Error> {
    let mut dm = DeviceManager {
        devices: Vec::new(),
        block_devices: Vec::new(),
    };
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
        let device = create_ide_controller(*device, &mut dm)?;
        dm.register_device(Arc::new(Mutex::new(device)));
    }

    Ok(dm)
}

type DeviceWrapped = Arc<Mutex<dyn Device>>;

pub trait Device: core::fmt::Debug + Send + Sync {
    fn children(&self) -> Option<Box<dyn Iterator<Item = DeviceWrapped>>>;

    fn print_device(&self) {
        println!("device: {self:#?}");

        if let Some(children) = self.children() {
            for device in children {
                #[allow(clippy::redundant_closure_for_method_calls)] // false positive submit pr
                device.with_ref(|d| d.print_device());
            }
        }
    }

    fn as_block_device(&mut self) -> Option<&mut dyn BlockDevice> {
        None
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

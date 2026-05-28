use crate::device_manager::DeviceManager;
use crate::pci::ide::structs::Drive;
use crate::pci::ide::structs::DriveType;
use crate::pci::ide::structs::IdentificationSpaceRaw;
use crate::pci::ide::structs::Status;
use crate::pci::ide::structs::{Channel, Command};
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::ascii::Char;

use anyhow::{Error, bail};

use crate::device_manager::BlockDevice;
use crate::multitasking::mutex::Mutex;
use crate::pci::ide::structs::HddSelect;
use crate::timer::sleep;
use crate::{device_manager::Device, pci::DeviceInfo};

mod structs;

#[derive(Debug)]
pub struct IdeController {
    drive_1: Option<Arc<Mutex<dyn BlockDevice>>>,
    drive_2: Option<Arc<Mutex<dyn BlockDevice>>>,
    drive_3: Option<Arc<Mutex<dyn BlockDevice>>>,
    drive_4: Option<Arc<Mutex<dyn BlockDevice>>>,
}

impl Device for IdeController {
    fn children(&self) -> Option<Box<dyn Iterator<Item = Arc<Mutex<dyn Device>>>>> {
        Some(Box::new(
            [
                self.drive_1.clone(),
                self.drive_2.clone(),
                self.drive_3.clone(),
                self.drive_4.clone(),
            ]
            .into_iter()
            .flatten()
            .map(|x| x as Arc<Mutex<dyn Device>>),
        ))
    }
}

impl Device for Drive {
    fn children(&self) -> Option<Box<dyn Iterator<Item = Arc<Mutex<dyn Device>>>>> {
        None
    }

    fn as_block_device(&mut self) -> Option<&mut dyn BlockDevice> {
        Some(self)
    }
}

impl BlockDevice for Drive {
    fn read_sectors(&mut self, lba: u64, count: u8, buffer: &mut [u8]) -> Result<(), Error> {
        ide_read_sectors(self, count, lba, buffer)
    }

    #[allow(unused)]
    fn write_sectors(&mut self, lba: u64, count: u8, buffer: &[u8]) -> Result<(), Error> {
        todo!()
    }

    fn total_sectors(&self) -> u64 {
        self.size
    }
}

#[derive(thiserror::Error, Debug)]
pub enum IdeCreationError {
    #[error("One or more channels were in pci native mode")]
    PciNativeMode,
}
/// # Errors
/// Will return `IdeCreationError::PciNativeMode` if the ide controller is in pci native mode
pub fn create_ide_controller(
    pci_device: DeviceInfo,
    device_manager: &mut DeviceManager,
) -> Result<IdeController, IdeCreationError> {
    fn setup_drive(
        drive: Drive,
        device_manager: &mut DeviceManager,
    ) -> Arc<Mutex<dyn BlockDevice>> {
        let drive = Arc::new(Mutex::new(drive));

        device_manager.register_device(drive.clone() as Arc<Mutex<dyn Device>>);

        device_manager.register_block_device(drive.clone() as Arc<Mutex<dyn BlockDevice>>);

        drive as Arc<Mutex<dyn BlockDevice>>
    }

    // Pci native mode is unsupported
    if pci_device.prog_if.pci_native_mode_1() || pci_device.prog_if.pci_native_mode_2() {
        return Err(IdeCreationError::PciNativeMode);
    }

    let primary_channel = unsafe { Channel::new(0x1F0, 0x3F6) };
    let sec_channel = unsafe { Channel::new(0x170, 0x376) };

    let (drive_1, drive_2) = init_channel(primary_channel);
    let (drive_3, drive_4) = init_channel(sec_channel);

    Ok(IdeController {
        drive_1: drive_1.map(|drive| setup_drive(drive, device_manager)),
        drive_2: drive_2.map(|drive| setup_drive(drive, device_manager)),
        drive_3: drive_3.map(|drive| setup_drive(drive, device_manager)),
        drive_4: drive_4.map(|drive| setup_drive(drive, device_manager)),
    })
}

fn init_channel(mut channel: Channel) -> (Option<Drive>, Option<Drive>) {
    channel.write_control(2);

    let channel = Arc::new(Mutex::new(channel));

    (
        init_drive(&channel, DriveType::Parent),
        init_drive(&channel, DriveType::Child),
    )
}

fn init_drive(channel_lock: &Arc<Mutex<Channel>>, drive_type: DriveType) -> Option<Drive> {
    channel_lock.with_mut_ref(|channel| {
        channel.write_hdd_sel(HddSelect::new().with_child(drive_type == DriveType::Child));
    });
    sleep(1);

    channel_lock.with_mut_ref(|channel| channel.send_command(Command::Identify));
    sleep(1);

    let mut channel = channel_lock.acquire();

    loop {
        let status = channel.get_status_reg();

        if Status::from_bits(0) == status || status.error() {
            return None;
        }

        if !status.busy() && status.data_request_ready() {
            break;
        }
    }

    let mut buffer: [u16; 256] = [0; 256];

    channel.read_ident_space(&mut buffer);

    let maybe = (&raw const buffer).cast::<IdentificationSpaceRaw>();

    let mut ident_space = unsafe { maybe.read_unaligned() };

    // TODO: Turn this into a builder
    let mut drive = Drive {
        channel: channel_lock.clone(),
        drive: drive_type,
        signature: ident_space.general_configuration,
        caps: ident_space.capabilities,
        command_set: ident_space.command_sets_enabled,
        size: 0, // unknown
        model: [Char::Null; 41],
    };

    if drive.command_set & (1 << 26) != 0 {
        drive.size = u64::from(ident_space.lba28_total_sectors);
    } else {
        drive.size = ident_space.lba48_total_sectors;
    }

    ident_space
        .model_number
        .chunks_exact_mut(2)
        .flat_map(|chunk| {
            chunk.swap(0, 1);
            chunk
        })
        .map(|x| Char::from_u8(*x).unwrap_or(Char::QuotationMark))
        .enumerate()
        .for_each(|(i, x)| drive.model[i] = x);

    drive.model[40] = Char::Null;

    Some(drive)
}

fn ide_read_sectors(
    drive: &Drive,
    num_of_sectors: u8,
    lba: u64,
    buffer: &mut [u8],
) -> Result<(), Error> {
    assert!((lba + u64::from(num_of_sectors)) <= drive.size);

    let lba_0: u8 = (lba & 0xFF).try_into().expect("Should be only one byte"); // byte 1

    let lba_1: u8 = ((lba & 0xFF00) >> 8)
        .try_into()
        .expect("Should be only one byte"); // byte 2

    let lba_2: u8 = ((lba & 0x00FF_0000) >> 16)
        .try_into()
        .expect("Should be only one byte"); // byte 3

    let head: u8 = ((lba & 0x0F00_0000) >> 24)
        .try_into()
        .expect("Should be only one byte"); // head

    let mut channel = drive.channel.acquire();

    while channel.get_status_reg().busy() {} // wait until idlea

    channel.write_hdd_sel(
        HddSelect::new()
            .with_child(drive.drive == DriveType::Child)
            .with_lba_mode(true)
            .with_head_or_lba_high(head),
    );

    channel.write_sec_count_0(num_of_sectors);
    channel.write_lba_0(lba_0);
    channel.write_lba_1(lba_1);
    channel.write_lba_2(lba_2);

    channel.send_command(Command::ReadPio);

    let mut words_read = 0;

    for sector in buffer.chunks_mut(512).take(num_of_sectors.into()) {
        // split the buffer into sectors
        poll_ide(&mut channel)?;

        for buffer_word in sector.chunks_exact_mut(2) {
            let word = unsafe { channel.data_reg.read() };

            buffer_word[0] = (word & 0xff).try_into().expect("should be only one byte"); // byte 1
            buffer_word[1] = (word >> 8).try_into().expect("should be only one byte"); // byte 1

            words_read += 1;
        }
    }

    for _ in (words_read % 256)..256 {
        unsafe {
            channel.data_reg.read();
        }
    }

    Ok(())
}

fn poll_ide(channel: &mut Channel) -> Result<(), Error> {
    while channel.get_status_reg().busy() {}

    let status = channel.get_status_reg();

    if status.error() {
        bail!("while polling error occured");
    }

    if status.drive_write_failed() {
        bail!("drive write failed");
    }

    if !status.data_request_ready() {
        bail!("drive not ready for new data");
    }

    Ok(())
}

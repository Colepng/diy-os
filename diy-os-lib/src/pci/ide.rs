use core::{ascii::Char, ffi::c_char};

use anyhow::{Error, bail};
use bitfield_struct::bitfield;
use volatile::access::ReadOnly;
use x86_64::{
    instructions::port::{Port, PortReadOnly, PortWriteOnly, ReadWriteAccess},
    structures::port::PortRead,
};

use crate::{
    multitasking::{exit, sleep},
    pci::DeviceInfo,
    print, println,
    timer::{Duration, Miliseconds, Nanoseconds},
};

pub fn init(ide_controller: DeviceInfo) -> ! {
    if ide_controller.prog_if.pci_native_mode_1() || ide_controller.prog_if.pci_native_mode_2() {
        panic!("uhhh both should be in compatibility mode");
    }

    let mut drives: [Drive; 4];
    let mut buffer: [u16; 256] = [0; 256];

    // thus since both are in compatibility mode, we can assume those io ports
    let mut primary_channel = unsafe { Channel::new(0x1F0, 0x3F6) };
    let mut sec_channel = unsafe { Channel::new(0x170, 0x376) };

    // disable iqrs
    primary_channel.write_control(2);
    sec_channel.write_control(2);

    primary_channel.write_hdd_sel(0xB0);
    sleep(Nanoseconds(400).into());

    primary_channel.send_command(Command::Identify);
    sleep(Nanoseconds(400).into());

    loop {
        let status = primary_channel.get_status_reg();

        if status.error() {
            panic!("not an ata drive");
            break;
        }

        if !status.busy() && status.data_request_ready() {
            break;
        }
    }

    primary_channel.read_ident_space(&mut buffer);

    let maybe =
        unsafe { core::mem::transmute::<&[u16; 256], *const IdentificationSpaceRaw>(&buffer) };

    let mut ident_space = unsafe { maybe.read_unaligned() };

    let mut drive = Drive {
        exists: true,
        channel: ChannelType::Primary,
        drive: DriveType::Parent,
        signature: ident_space.general_configuration,
        caps: ident_space.capabilities,
        command_set: ident_space.command_sets_enabled,
        size: 0, // unknown
        model: [Char::Null; 41],
    };

    if drive.command_set & (1 << 26) != 0 {
        drive.size = ident_space.lba28_total_sectors as u64;
    } else {
        drive.size = ident_space.lba48_total_sectors;
    }

    ident_space
        .model_number
        .chunks_exact_mut(2)
        .map(|chunk| {
            chunk.swap(0, 1);
            chunk
        })
        .flatten()
        .map(|x| Char::from_u8(*x).unwrap_or(Char::QuotationMark))
        .enumerate()
        .for_each(|(i, x)| drive.model[i] = x);

    drive.model[40] = Char::Null;

    println!("drive: {:#?}", drive);

    drive.model.iter().for_each(|a| print!("{a}"));
    print!("\n");

    // loop {}
    unsafe { exit() }

    // sec_channel.write_control(2);

    // println!("err: {:#?}", primary_channel.get_err_reg());
    // println!("status: {:#?}", primary_channel.get_status_reg());
    // let channel_2: Port<u32> = Port::new(0x1F0); // is the first one
}

fn ide_read_sectors(
    drive: &mut Drive,
    num_of_sectors: u8,
    lba: u64,
    buffer: &mut [u8],
    channel: &mut Channel,
) -> Result<(), Error> {
    assert!((lba as u64 + num_of_sectors as u64) <= drive.size);

    let lba_0: u8 = (lba & 0xFF >> 0)
        .try_into()
        .expect("Should be only one byte"); // byte 1

    let lba_1: u8 = (lba & 0xFF00 >> 8)
        .try_into()
        .expect("Should be only one byte"); // byte 2

    let lba_2: u8 = (lba & 0xFF0000 >> 16)
        .try_into()
        .expect("Should be only one byte"); // byte 3

    let head: u8 = (lba & 0xF000000 >> 24)
        .try_into()
        .expect("Should be only one byte"); // head

    while channel.get_status_reg().busy() {} // wait until idle

    channel.write_hdd_sel(0xF0 | head); // 0xf0, means slave drive and lba mode, or with head as
    // bottom 4 bit is used for addressing

    channel.write_sec_count_0(num_of_sectors);
    channel.write_lba_0(lba_0);
    channel.write_lba_1(lba_1);
    channel.write_lba_2(lba_2);

    for sector in buffer.chunks_exact_mut(256).take(num_of_sectors.into()) {
        // split the buffer into sectors
        poll_ide(channel)?;

        for buffer_word in sector.chunks_exact_mut(2) {
            let word = unsafe { channel.data_reg.read() };

            buffer_word[0] = (word & 0xff).try_into().expect("should be only one byte"); // byte 1
            buffer_word[1] = (word >> 8).try_into().expect("should be only one byte"); // byte 1
        }
    }

    Ok(())
}

pub enum Mode {
    Read,
    Write,
}

unsafe fn ide_ata_raw_access(
    mode: Mode,
    drive: &mut Drive,
    lba_addr: u32,
    num_of_sectors: u8,
    selector: u16,
    buffer: *mut u8,
) {
}

fn poll_ide(mut channel: &mut Channel) -> Result<(), Error> {
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

//
// #[repr(C, packed)]
// struct IdentificationSpaceRaw {
//     device_type: u16,
//     cylinders: u32,
//     heads_1: u32,
//     heads_2: u16,
//     sectors: u64,
//     serial: [u8; 34],
//     model: [u8; 41],
//     idk_bruh_part_5: [u8; 3],
//     caps: u16,
//     _idk_bruh: [u8; 6],
//     field_valid: [u8; 14],
//     ident_max_lba: u32,
//     _idk_part_3: [u8; 40],
//     ident_commmand_sets: u32,
//     _idk_part_2: [u8; 32],
//     lba_ext: u32,
//     _idk_part_4: [u8; 308],
//     // PLS GIVE ME THE IDE DOCS, i hate you os dev wiki
// }

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdentificationSpaceRaw {
    /// Word 0: General configuration bitfield
    general_configuration: u16,
    /// Word 1: Number of logical cylinders (CHS, legacy)
    logical_cylinders: u16,
    /// Word 2: Specific configuration
    specific_configuration: u16,
    /// Word 3: Number of logical heads (CHS, legacy)
    logical_heads: u16,
    /// Words 4-5: Retired vendor-specific bytes
    retired_vendor_specific_1: [u16; 2],
    /// Word 6: Number of logical sectors per track (CHS, legacy)
    logical_sectors_per_track: u16,
    /// Words 7-8: Reserved for CompactFlash association
    compactflash_reserved: [u16; 2],
    /// Word 9: Retired vendor-specific
    retired_vendor_specific_2: u16,
    /// Words 10-19: Serial number (ASCII, byte-swapped within each word)
    serial_number: [u8; 20],
    /// Word 20: Retired buffer type
    retired_buffer_type: u16,
    /// Word 21: Retired buffer size in 512-byte units
    retired_buffer_size: u16,
    /// Word 22: Obsolete ECC bytes count
    obsolete_ecc_bytes: u16,
    /// Words 23-26: Firmware revision (ASCII, byte-swapped within each word)
    firmware_revision: [u8; 8],
    /// Words 27-46: Model number (ASCII, byte-swapped within each word)
    model_number: [u8; 40],
    /// Word 47: Maximum sectors transferred per interrupt on READ/WRITE MULTIPLE
    max_sectors_per_multiple_transfer: u16,
    /// Word 48: Trusted computing feature set
    trusted_computing_features: u16,
    /// Word 49: Capabilities (LBA support, DMA support, IORDY)
    capabilities: u16,
    /// Word 50: Capabilities extended (standby timer minimum)
    capabilities_extended: u16,
    /// Word 51: PIO data transfer cycle timing (legacy)
    pio_transfer_timing: u16,
    /// Word 52: DMA data transfer cycle timing (legacy)
    dma_transfer_timing: u16,
    /// Word 53: Field validity flags (which later words are valid)
    field_validity: u16,
    /// Word 54: Current number of logical cylinders
    current_logical_cylinders: u16,
    /// Word 55: Current number of logical heads
    current_logical_heads: u16,
    /// Word 56: Current number of logical sectors per track
    current_logical_sectors_per_track: u16,
    /// Words 57-58: Current capacity in CHS-addressable sectors
    current_chs_capacity: u32,
    /// Word 59: Multi-sector setting (sectors per interrupt currently configured)
    current_multi_sector_setting: u16,
    /// Words 60-61: Total user-addressable sectors in LBA28 mode
    lba28_total_sectors: u32,
    /// Word 62: Obsolete single-word DMA modes
    single_word_dma_modes: u16,
    /// Word 63: Multiword DMA modes supported and selected
    multiword_dma_modes: u16,
    /// Word 64: Advanced PIO modes supported (PIO 3, PIO 4)
    advanced_pio_modes: u16,
    /// Word 65: Minimum multiword DMA cycle time per word (ns)
    min_multiword_dma_cycle_time: u16,
    /// Word 66: Manufacturer-recommended multiword DMA cycle time (ns)
    recommended_multiword_dma_cycle_time: u16,
    /// Word 67: Minimum PIO cycle time without flow control (ns)
    min_pio_cycle_time_no_iordy: u16,
    /// Word 68: Minimum PIO cycle time with IORDY flow control (ns)
    min_pio_cycle_time_with_iordy: u16,
    /// Words 69-70: Reserved for identify packet device
    identify_packet_reserved: [u16; 2],
    /// Words 71-74: Reserved for ATAPI command processing
    atapi_command_reserved: [u16; 4],
    /// Word 75: Maximum queue depth (NCQ) minus one
    max_queue_depth: u16,
    /// Words 76-79: Serial ATA capabilities and features
    serial_ata_capabilities: [u16; 4],
    /// Word 80: Major version number (ATA standard bitmap)
    major_version: u16,
    /// Word 81: Minor version number (specific draft revision)
    minor_version: u16,
    /// Words 82-83: Command sets and feature sets supported (bit 26 = LBA48 supported)
    command_sets_supported: u32,
    /// Word 84: Command sets and feature sets supported, extended
    command_sets_supported_extended: u16,
    /// Words 85-86: Command sets and feature sets currently enabled
    command_sets_enabled: u32,
    /// Word 87: Command sets and feature sets currently enabled, extended
    command_sets_enabled_extended: u16,
    /// Word 88: Ultra DMA modes supported and selected
    ultra_dma_modes: u16,
    /// Word 89: Time required for SECURITY ERASE UNIT completion
    security_erase_time: u16,
    /// Word 90: Time required for ENHANCED SECURITY ERASE UNIT completion
    enhanced_security_erase_time: u16,
    /// Word 91: Current advanced power management value
    current_apm_value: u16,
    /// Word 92: Master password identifier
    master_password_identifier: u16,
    /// Word 93: Hardware reset result (PATA)
    hardware_reset_result: u16,
    /// Word 94: Current automatic acoustic management value
    current_acoustic_management_value: u16,
    /// Word 95: Stream minimum request size
    stream_minimum_request_size: u16,
    /// Word 96: Streaming transfer time in DMA
    streaming_transfer_time_dma: u16,
    /// Word 97: Streaming access latency in DMA and PIO
    streaming_access_latency: u16,
    /// Words 98-99: Streaming performance granularity
    streaming_performance_granularity: u32,
    /// Words 100-103: Total user-addressable sectors in LBA48 mode
    lba48_total_sectors: u64,
    /// Word 104: Streaming transfer time in PIO
    streaming_transfer_time_pio: u16,
    /// Word 105: Maximum number of 512-byte blocks per DATA SET MANAGEMENT command
    max_blocks_per_data_set_management: u16,
    /// Word 106: Physical and logical sector size information
    physical_logical_sector_size_info: u16,
    /// Word 107: Inter-seek delay for ISO 7779 acoustic testing (microseconds)
    inter_seek_delay: u16,
    /// Words 108-111: World Wide Name (64-bit unique device identifier)
    world_wide_name: u64,
    /// Words 112-115: Reserved for World Wide Name extension
    world_wide_name_extension_reserved: [u16; 4],
    /// Word 116: Reserved for technical report
    technical_report_reserved: u16,
    /// Words 117-118: Logical sector size in words (when sectors are not 512 bytes)
    logical_sector_size_in_words: u32,
    /// Word 119: Commands and feature sets supported (third extension)
    commands_supported_extension_3: u16,
    /// Word 120: Commands and feature sets enabled (third extension)
    commands_enabled_extension_3: u16,
    /// Words 121-126: Reserved for expanded supported and enabled settings
    expanded_settings_reserved: [u16; 6],
    /// Word 127: Removable media status notification feature set
    removable_media_status_notification: u16,
    /// Word 128: Security status
    security_status: u16,
    /// Words 129-159: Vendor-specific
    vendor_specific: [u16; 31],
    /// Word 160: CompactFlash association power mode
    cfa_power_mode: u16,
    /// Words 161-167: Reserved for CompactFlash association
    cfa_reserved_block_1: [u16; 7],
    /// Word 168: Device nominal form factor
    device_nominal_form_factor: u16,
    /// Word 169: DATA SET MANAGEMENT command support
    data_set_management_support: u16,
    /// Words 170-173: Additional product identifier
    additional_product_identifier: [u8; 8],
    /// Words 174-175: Reserved
    cfa_reserved_block_2: [u16; 2],
    /// Words 176-205: Current media serial number (ASCII)
    current_media_serial_number: [u8; 60],
    /// Word 206: SCT command transport
    sct_command_transport: u16,
    /// Words 207-208: Reserved for CE-ATA
    ce_ata_reserved: [u16; 2],
    /// Word 209: Alignment of logical blocks within physical block
    logical_block_alignment: u16,
    /// Words 210-211: Write-Read-Verify sector mode 3 count
    write_read_verify_mode_3_count: u32,
    /// Words 212-213: Write-Read-Verify sector mode 2 count
    write_read_verify_mode_2_count: u32,
    /// Words 214-216: Non-volatile cache capabilities
    nv_cache_capabilities: [u16; 3],
    /// Word 217: Nominal media rotation rate (1 = SSD, otherwise RPM)
    nominal_rotation_rate: u16,
    /// Word 218: Reserved
    rotation_rate_reserved: u16,
    /// Word 219: Non-volatile cache options
    nv_cache_options: u16,
    /// Word 220: Write-Read-Verify feature set current mode
    write_read_verify_current_mode: u16,
    /// Word 221: Reserved
    write_read_verify_reserved: u16,
    /// Word 222: Transport major version number
    transport_major_version: u16,
    /// Word 223: Transport minor version number
    transport_minor_version: u16,
    /// Words 224-229: Reserved for CE-ATA
    ce_ata_reserved_2: [u16; 6],
    /// Words 230-233: Extended number of user-addressable sectors
    extended_user_addressable_sectors: u64,
    /// Word 234: Minimum number of 512-byte blocks per DOWNLOAD MICROCODE
    min_blocks_per_download_microcode: u16,
    /// Word 235: Maximum number of 512-byte blocks per DOWNLOAD MICROCODE
    max_blocks_per_download_microcode: u16,
    /// Words 236-254: Reserved
    final_reserved: [u16; 19],
    /// Word 255: Integrity word (signature byte + checksum byte)
    integrity_word: u16,
}

// Compile-time check that the struct is exactly 512 bytes (256 words)
const _: () = assert!(core::mem::size_of::<IdentificationSpaceRaw>() == 512);

// Compile-time check that the struct is exactly 512 bytes (256 words)
const _: () = assert!(core::mem::size_of::<IdentificationSpaceRaw>() == 512);

struct IdentificationSpace {
    device_type: u16,
    cylinders: u32,
    heads: u64,
}

#[derive(Debug)]
enum ChannelType {
    Primary,
    Secondary,
}

#[derive(Debug)]
enum DriveType {
    Parent,
    Child,
}

#[derive(Debug)]
struct Drive {
    exists: bool,
    channel: ChannelType,
    drive: DriveType,
    signature: u16,
    caps: u16,
    command_set: u32,
    size: u64,
    model: [Char; 41],
}

#[repr(u8)]
enum Command {
    ReadPio = 0x20,
    ReadPioExt = 0x24,
    ReadDma = 0xC8,
    ReadDmaExt = 0x25,
    WritePio = 0x30,
    WritePioExt = 0x34,
    WrtieDma = 0xCA,
    WriteDmaExt = 0x35,
    CacheFlush = 0xE7,
    CacheFlushExt = 0xEA,
    Packet = 0xA0,
    IdentifyPacket = 0xA1,
    Identify = 0xEC,
}

struct Channel {
    data_reg: Port<u16>,
    err_reg: PortReadOnly<u8>,
    feat_reg: PortWriteOnly<u8>,
    set_count_0: Port<u8>,
    lba_0: Port<u8>,
    lba_1: Port<u8>,
    lba_2: Port<u8>,
    hdd_select: Port<u8>,
    commmand_reg: PortWriteOnly<u8>,
    status_reg: PortReadOnly<u8>,
    alt_status_reg: PortReadOnly<u8>,
    ctrl_reg: PortWriteOnly<u8>,
    // dev_addr unused
}

impl Channel {
    /// Caller must make sure that this port is the base io port for the primary channel
    unsafe fn new(base_port: u16, ctrl_port: u16) -> Self {
        Self {
            data_reg: Port::new(base_port),
            err_reg: PortReadOnly::new(base_port + 1),
            feat_reg: PortWriteOnly::new(base_port + 1),
            set_count_0: Port::new(base_port + 2),
            lba_0: Port::new(base_port + 3),
            lba_1: Port::new(base_port + 4),
            lba_2: Port::new(base_port + 5),
            hdd_select: Port::new(base_port + 6),
            commmand_reg: PortWriteOnly::new(base_port + 7),
            status_reg: PortReadOnly::new(base_port + 7),
            alt_status_reg: PortReadOnly::new(ctrl_port + 2),
            ctrl_reg: PortWriteOnly::new(ctrl_port + 2),
        }
    }

    fn get_err_reg(&mut self) -> ErrRaw {
        let bits = unsafe { self.err_reg.read() };

        ErrRaw::from_bits(bits)
    }

    fn get_status_reg(&mut self) -> Status {
        let bits = unsafe { self.status_reg.read() };

        Status::from_bits(bits)
    }

    fn write_control(&mut self, value: u8) {
        unsafe { self.ctrl_reg.write(value) };
    }

    fn write_hdd_sel(&mut self, value: u8) {
        unsafe { self.hdd_select.write(value) };
    }

    fn write_lba_0(&mut self, value: u8) {
        unsafe { self.lba_0.write(value) };
    }

    fn write_lba_1(&mut self, value: u8) {
        unsafe { self.lba_1.write(value) };
    }

    fn write_lba_2(&mut self, value: u8) {
        unsafe { self.lba_2.write(value) };
    }

    fn write_sec_count_0(&mut self, value: u8) {
        unsafe { self.set_count_0.write(value) };
    }

    fn send_command(&mut self, cmd: Command) {
        unsafe { self.commmand_reg.write(cmd as u8) };
    }

    fn read_ident_space(&mut self, buffer: &mut [u16; 256]) {
        for i in 0..256 {
            buffer[i] = unsafe { self.data_reg.read() };
        }
    }
}

#[bitfield(u8)]
pub struct ErrRaw {
    #[bits(1)]
    no_address_mark: bool,
    #[bits(1)]
    track_0_not_found: bool,
    #[bits(1)]
    command_aborted: bool,
    #[bits(1)]
    media_change_request: bool,
    #[bits(1)]
    id_mark_not_found: bool,
    #[bits(1)]
    media_changed: bool,
    #[bits(1)]
    uncorrectable_data: bool,
    #[bits(1)]
    bad_block: bool,
}

#[bitfield(u8)]
pub struct Status {
    #[bits(1)]
    error: bool,
    #[bits(1)]
    index: bool,
    #[bits(1)]
    corrected_data: bool,
    #[bits(1)]
    data_request_ready: bool,
    #[bits(1)]
    drive_seek_complete: bool,
    #[bits(1)]
    drive_write_failed: bool,
    #[bits(1)]
    drive_ready: bool,
    #[bits(1)]
    busy: bool,
}

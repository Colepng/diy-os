use alloc::sync::Arc;
use bitfield_struct::bitfield;
use core::ascii::Char;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

use crate::multitasking::mutex::Mutex;

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
#[allow(unused)]
pub(super) struct IdentificationSpaceRaw {
    /// Word 0: General configuration bitfield
    pub(super) general_configuration: u16,
    /// Word 1: Number of logical cylinders (CHS, legacy)
    pub(super) logical_cylinders: u16,
    /// Word 2: Specific configuration
    pub(super) specific_configuration: u16,
    /// Word 3: Number of logical heads (CHS, legacy)
    pub(super) logical_heads: u16,
    /// Words 4-5: Retired vendor-specific bytes
    pub(super) retired_vendor_specific_1: [u16; 2],
    /// Word 6: Number of logical sectors per track (CHS, legacy)
    pub(super) logical_sectors_per_track: u16,
    /// Words 7-8: Reserved for Compact Flash association
    pub(super) compactflash_reserved: [u16; 2],
    /// Word 9: Retired vendor-specific
    pub(super) retired_vendor_specific_2: u16,
    /// Words 10-19: Serial number (ASCII, byte-swapped within each word)
    pub(super) serial_number: [u8; 20],
    /// Word 20: Retired buffer type
    pub(super) retired_buffer_type: u16,
    /// Word 21: Retired buffer size in 512-byte units
    pub(super) retired_buffer_size: u16,
    /// Word 22: Obsolete ECC bytes count
    pub(super) obsolete_ecc_bytes: u16,
    /// Words 23-26: Firmware revision (ASCII, byte-swapped within each word)
    pub(super) firmware_revision: [u8; 8],
    /// Words 27-46: Model number (ASCII, byte-swapped within each word)
    pub(super) model_number: [u8; 40],
    /// Word 47: Maximum sectors transferred per interrupt on READ/WRITE MULTIPLE
    pub(super) max_sectors_per_multiple_transfer: u16,
    /// Word 48: Trusted computing feature set
    pub(super) trusted_computing_features: u16,
    /// Word 49: Capabilities (LBA support, DMA support, IORDY)
    pub(super) capabilities: u16,
    /// Word 50: Capabilities extended (standby timer minimum)
    pub(super) capabilities_extended: u16,
    /// Word 51: PIO data transfer cycle timing (legacy)
    pub(super) pio_transfer_timing: u16,
    /// Word 52: DMA data transfer cycle timing (legacy)
    pub(super) dma_transfer_timing: u16,
    /// Word 53: Field validity flags (which later words are valid)
    pub(super) field_validity: u16,
    /// Word 54: Current number of logical cylinders
    pub(super) current_logical_cylinders: u16,
    /// Word 55: Current number of logical heads
    pub(super) current_logical_heads: u16,
    /// Word 56: Current number of logical sectors per track
    pub(super) current_logical_sectors_per_track: u16,
    /// Words 57-58: Current capacity in CHS-addressable sectors
    pub(super) current_chs_capacity: u32,
    /// Word 59: Multi-sector setting (sectors per interrupt currently configured)
    pub(super) current_multi_sector_setting: u16,
    /// Words 60-61: Total user-addressable sectors in LBA28 mode
    pub(super) lba28_total_sectors: u32,
    /// Word 62: Obsolete single-word DMA modes
    pub(super) single_word_dma_modes: u16,
    /// Word 63: Multiword DMA modes supported and selected
    pub(super) multiword_dma_modes: u16,
    /// Word 64: Advanced PIO modes supported (PIO 3, PIO 4)
    pub(super) advanced_pio_modes: u16,
    /// Word 65: Minimum multiword DMA cycle time per word (ns)
    pub(super) min_multiword_dma_cycle_time: u16,
    /// Word 66: Manufacturer-recommended multiword DMA cycle time (ns)
    pub(super) recommended_multiword_dma_cycle_time: u16,
    /// Word 67: Minimum PIO cycle time without flow control (ns)
    pub(super) min_pio_cycle_time_no_iordy: u16,
    /// Word 68: Minimum PIO cycle time with IORDY flow control (ns)
    pub(super) min_pio_cycle_time_with_iordy: u16,
    /// Words 69-70: Reserved for identify packet device
    pub(super) identify_packet_reserved: [u16; 2],
    /// Words 71-74: Reserved for ATAPI command processing
    pub(super) atapi_command_reserved: [u16; 4],
    /// Word 75: Maximum queue depth (NCQ) minus one
    pub(super) max_queue_depth: u16,
    /// Words 76-79: Serial ATA capabilities and features
    pub(super) serial_ata_capabilities: [u16; 4],
    /// Word 80: Major version number (ATA standard bitmap)
    pub(super) major_version: u16,
    /// Word 81: Minor version number (specific draft revision)
    pub(super) minor_version: u16,
    /// Words 82-83: Command sets and feature sets supported (bit 26 = LBA48 supported)
    pub(super) command_sets_supported: u32,
    /// Word 84: Command sets and feature sets supported, extended
    pub(super) command_sets_supported_extended: u16,
    /// Words 85-86: Command sets and feature sets currently enabled
    pub(super) command_sets_enabled: u32,
    /// Word 87: Command sets and feature sets currently enabled, extended
    pub(super) command_sets_enabled_extended: u16,
    /// Word 88: Ultra DMA modes supported and selected
    pub(super) ultra_dma_modes: u16,
    /// Word 89: Time required for SECURITY ERASE UNIT completion
    pub(super) security_erase_time: u16,
    /// Word 90: Time required for ENHANCED SECURITY ERASE UNIT completion
    pub(super) enhanced_security_erase_time: u16,
    /// Word 91: Current advanced power management value
    pub(super) current_apm_value: u16,
    /// Word 92: Master password identifier
    pub(super) master_password_identifier: u16,
    /// Word 93: Hardware reset result (PATA)
    pub(super) hardware_reset_result: u16,
    /// Word 94: Current automatic acoustic management value
    pub(super) current_acoustic_management_value: u16,
    /// Word 95: Stream minimum request size
    pub(super) stream_minimum_request_size: u16,
    /// Word 96: Streaming transfer time in DMA
    pub(super) streaming_transfer_time_dma: u16,
    /// Word 97: Streaming access latency in DMA and PIO
    pub(super) streaming_access_latency: u16,
    /// Words 98-99: Streaming performance granularity
    pub(super) streaming_performance_granularity: u32,
    /// Words 100-103: Total user-addressable sectors in LBA48 mode
    pub(super) lba48_total_sectors: u64,
    /// Word 104: Streaming transfer time in PIO
    pub(super) streaming_transfer_time_pio: u16,
    /// Word 105: Maximum number of 512-byte blocks per DATA SET MANAGEMENT command
    pub(super) max_blocks_per_data_set_management: u16,
    /// Word 106: Physical and logical sector size information
    pub(super) physical_logical_sector_size_info: u16,
    /// Word 107: Inter-seek delay for ISO 7779 acoustic testing (microseconds)
    pub(super) inter_seek_delay: u16,
    /// Words 108-111: World Wide Name (64-bit unique device identifier)
    pub(super) world_wide_name: u64,
    /// Words 112-115: Reserved for World Wide Name extension
    pub(super) world_wide_name_extension_reserved: [u16; 4],
    /// Word 116: Reserved for technical report
    pub(super) technical_report_reserved: u16,
    /// Words 117-118: Logical sector size in words (when sectors are not 512 bytes)
    pub(super) logical_sector_size_in_words: u32,
    /// Word 119: Commands and feature sets supported (third extension)
    pub(super) commands_supported_extension_3: u16,
    /// Word 120: Commands and feature sets enabled (third extension)
    pub(super) commands_enabled_extension_3: u16,
    /// Words 121-126: Reserved for expanded supported and enabled settings
    pub(super) expanded_settings_reserved: [u16; 6],
    /// Word 127: Removable media status notification feature set
    pub(super) removable_media_status_notification: u16,
    /// Word 128: Security status
    pub(super) security_status: u16,
    /// Words 129-159: Vendor-specific
    pub(super) vendor_specific: [u16; 31],
    /// Word 160: Compact Flash association power mode
    pub(super) cfa_power_mode: u16,
    /// Words 161-167: Reserved for Compact Flash association
    pub(super) cfa_reserved_block_1: [u16; 7],
    /// Word 168: Device nominal form factor
    pub(super) device_nominal_form_factor: u16,
    /// Word 169: DATA SET MANAGEMENT command support
    pub(super) data_set_management_support: u16,
    /// Words 170-173: Additional product identifier
    pub(super) additional_product_identifier: [u8; 8],
    /// Words 174-175: Reserved
    pub(super) cfa_reserved_block_2: [u16; 2],
    /// Words 176-205: Current media serial number (ASCII)
    pub(super) current_media_serial_number: [u8; 60],
    /// Word 206: SCT command transport
    pub(super) sct_command_transport: u16,
    /// Words 207-208: Reserved for CE-ATA
    pub(super) ce_ata_reserved: [u16; 2],
    /// Word 209: Alignment of logical blocks within physical block
    pub(super) logical_block_alignment: u16,
    /// Words 210-211: Write-Read-Verify sector mode 3 count
    pub(super) write_read_verify_mode_3_count: u32,
    /// Words 212-213: Write-Read-Verify sector mode 2 count
    pub(super) write_read_verify_mode_2_count: u32,
    /// Words 214-216: Non-volatile cache capabilities
    pub(super) nv_cache_capabilities: [u16; 3],
    /// Word 217: Nominal media rotation rate (1 = SSD, otherwise RPM)
    pub(super) nominal_rotation_rate: u16,
    /// Word 218: Reserved
    pub(super) rotation_rate_reserved: u16,
    /// Word 219: Non-volatile cache options
    pub(super) nv_cache_options: u16,
    /// Word 220: Write-Read-Verify feature set current mode
    pub(super) write_read_verify_current_mode: u16,
    /// Word 221: Reserved
    pub(super) write_read_verify_reserved: u16,
    /// Word 222: Transport major version number
    pub(super) transport_major_version: u16,
    /// Word 223: Transport minor version number
    pub(super) transport_minor_version: u16,
    /// Words 224-229: Reserved for CE-ATA
    pub(super) ce_ata_reserved_2: [u16; 6],
    /// Words 230-233: Extended number of user-addressable sectors
    pub(super) extended_user_addressable_sectors: u64,
    /// Word 234: Minimum number of 512-byte blocks per DOWNLOAD MICROCODE
    pub(super) min_blocks_per_download_microcode: u16,
    /// Word 235: Maximum number of 512-byte blocks per DOWNLOAD MICROCODE
    pub(super) max_blocks_per_download_microcode: u16,
    /// Words 236-254: Reserved
    pub(super) final_reserved: [u16; 19],
    /// Word 255: Integrity word (signature byte + checksum byte)
    pub(super) integrity_word: u16,
}

// Compile-time check that the struct is exactly 512 bytes (256 words)
const _: () = assert!(core::mem::size_of::<IdentificationSpaceRaw>() == 512);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DriveType {
    Parent,
    Child,
}

#[derive(Debug)]
#[allow(unused)]
pub(super) struct Drive {
    pub(super) channel: Arc<Mutex<Channel>>,
    pub(super) drive: DriveType,
    pub(super) signature: u16,
    pub(super) caps: u16,
    pub(super) command_set: u32,
    pub(super) size: u64,
    pub(super) model: [Char; 41],
}

#[repr(u8)]
#[allow(unused)]
pub(super) enum Command {
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

#[derive(Debug)]
#[allow(unused)]
pub(super) struct Channel {
    pub(super) data_reg: Port<u16>,
    pub(super) err_reg: PortReadOnly<u8>,
    pub(super) feat_reg: PortWriteOnly<u8>,
    pub(super) set_count_0: Port<u8>,
    pub(super) lba_0: Port<u8>,
    pub(super) lba_1: Port<u8>,
    pub(super) lba_2: Port<u8>,
    pub(super) hdd_select: Port<u8>,
    pub(super) commmand_reg: PortWriteOnly<u8>,
    pub(super) status_reg: PortReadOnly<u8>,
    pub(super) alt_status_reg: PortReadOnly<u8>,
    pub(super) ctrl_reg: PortWriteOnly<u8>,
    // dev_addr unused
}

#[allow(unused)]
impl Channel {
    /// Caller must make sure that this port is the base io port for the primary channel
    pub(super) const unsafe fn new(base_port: u16, ctrl_port: u16) -> Self {
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

    pub(super) fn get_err_reg(&mut self) -> ErrRaw {
        let bits = unsafe { self.err_reg.read() };

        ErrRaw::from_bits(bits)
    }

    pub(super) fn get_status_reg(&mut self) -> Status {
        let bits = unsafe { self.status_reg.read() };

        Status::from_bits(bits)
    }

    pub(super) fn write_control(&mut self, value: u8) {
        unsafe { self.ctrl_reg.write(value) };
    }

    pub(super) fn write_hdd_sel(&mut self, value: HddSelect) {
        unsafe { self.hdd_select.write(value.into()) };
    }

    pub(super) fn write_lba_0(&mut self, value: u8) {
        unsafe { self.lba_0.write(value) };
    }

    pub(super) fn write_lba_1(&mut self, value: u8) {
        unsafe { self.lba_1.write(value) };
    }

    pub(super) fn write_lba_2(&mut self, value: u8) {
        unsafe { self.lba_2.write(value) };
    }

    pub(super) fn write_sec_count_0(&mut self, value: u8) {
        unsafe { self.set_count_0.write(value) };
    }

    pub(super) fn send_command(&mut self, cmd: Command) {
        unsafe { self.commmand_reg.write(cmd as u8) };
    }

    pub(super) fn read_ident_space(&mut self, buffer: &mut [u16; 256]) {
        for item in buffer.iter_mut().take(256) {
            *item = unsafe { self.data_reg.read() };
        }
    }
}

#[bitfield(u8)]
pub(super) struct HddSelect {
    /// Bits 0-3: Head number (CHS mode) or top 4 bits of LBA28 address (LBA mode)
    #[bits(4)]
    pub(super) head_or_lba_high: u8,

    /// Bit 4: Drive select. false = parent, true = child
    #[bits(1)]
    pub(super) child: bool,

    /// Bit 5: Always 1 (legacy "obsolete" bit)
    #[bits(1, default = true)]
    always_one_5: bool,

    /// Bit 6: Addressing mode. false = CHS, true = LBA
    #[bits(1)]
    pub(super) lba_mode: bool,

    /// Bit 7: Always 1 (legacy "obsolete" bit)
    #[bits(1, default = true)]
    always_one_7: bool,
}

#[bitfield(u8)]
pub struct ErrRaw {
    #[bits(1)]
    pub(super) no_address_mark: bool,
    #[bits(1)]
    pub(super) track_0_not_found: bool,
    #[bits(1)]
    pub(super) command_aborted: bool,
    #[bits(1)]
    pub(super) media_change_request: bool,
    #[bits(1)]
    pub(super) id_mark_not_found: bool,
    #[bits(1)]
    pub(super) media_changed: bool,
    #[bits(1)]
    pub(super) uncorrectable_data: bool,
    #[bits(1)]
    pub(super) bad_block: bool,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct Status {
    #[bits(1)]
    pub(super) error: bool,
    #[bits(1)]
    pub(super) index: bool,
    #[bits(1)]
    pub(super) corrected_data: bool,
    #[bits(1)]
    pub(super) data_request_ready: bool,
    #[bits(1)]
    pub(super) drive_seek_complete: bool,
    #[bits(1)]
    pub(super) drive_write_failed: bool,
    #[bits(1)]
    pub(super) drive_ready: bool,
    #[bits(1)]
    pub(super) busy: bool,
}

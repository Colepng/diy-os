use alloc::vec::Vec;
use x86_64::instructions::port;

use bitfield_struct::{bitenum, bitfield};

pub mod ide;
pub mod virtio;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigAddress {
    register_offset: u8,
    fn_dn_number: u8,
    bus_number: u8,
    res_enabled: u8,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub device_num: u16,
    pub command: CommandReg,
    pub status: u16,
    pub rev_id: u8,
    pub prog_if: IdeProgIf,
    pub subclass: MassStorageSubclass,
    pub class_code: ClassCode,
    pub cache_line_size: u8,
    pub lat_timer: u8,
    pub header_type: HeaderTypeInfo,
    pub bist: BistReg,
    pub header: Header,
}

#[bitfield(u8)]
pub struct IdeProgIf {
    #[bits(1)]
    pub pci_native_mode_1: bool,
    #[bits(1)]
    pub native_mode_w_1: bool,
    #[bits(1)]
    pub pci_native_mode_2: bool,
    #[bits(1)]
    pub native_mode_w_2: bool,
    #[bits(3)]
    _unused: u8,
    #[bits(1)]
    pub master_and_dma: bool,
}

#[bitenum(all = false)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassCode {
    Unclassified = 0x0,
    MassStorageController = 0x1,
    NetworkController = 0x2,
    DisplayController = 0x3,
    MultiMediaController = 0x4,
    MemoryController = 0x5,
    Bridge = 0x6,
    SimpleCommunicationController = 0x7,
    BaseSystemPeripheral = 0x8,
    InputDeviceController = 0x9,
    DockingStation = 0xA,
    Proccessor = 0xB,
    SerialBusController = 0xC,
    WirelessController = 0xD,
    IntelligentController = 0xE,
    SatelliteCommunicationController = 0xF,
    EncryptionController = 0x10,
    SignalProcessingController = 0x11,
    ProcessingAccelerator = 0x12,
    NonEssentialInstrumentation = 0x13,
    #[fallback]
    ReserverdOrOther,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[bitenum(all = false)]
pub enum MassStorageSubclass {
    ScsiBus = 0x00,
    Ide = 0x01,
    Floppy = 0x02,
    IpiBus = 0x03,
    Raid = 0x04,
    AtaWithAdma = 0x05,
    SerialAta = 0x06,
    SerialAttachedScsi = 0x07,
    NonVolatileMemory = 0x08,
    UniversalFlashStorage = 0x09,
    Other = 0x80,
    #[fallback]
    Unknown,
}

#[bitfield(u8)]
pub struct BistReg {
    #[bits(4)]
    pub comp_code: u8,
    #[bits(2)]
    _reserved: (),
    #[bits(1)]
    pub start_bist: u8,
    #[bits(1)]
    pub bist_cap: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Header {
    bar_addr: [u32; 6],
    cardbus_cis_ptr: u32,
    subsystem_vendor: u16,
    subsystem_id: u16,
    exp_rom_base_addr: u32,
    cap_ptr: u8,
    _reserved: [u8; 7],
    interrupt_line: u8,
    interrupt_pin: u8,
    min_grant: u8,
    max_lantecy: u8,
}

impl Header {
    pub fn get_bar(&self, index: u8) -> Bar {
        let raw_bits = self.bar_addr[index as usize];

        let raw = BarRaw::from_bits(raw_bits);

        if raw.io_space() {
            let io = IoSpaceRaw::from_bits(raw_bits);
            assert!(io.io_space());

            Bar::IoSpace { addr: io.addr() }
        } else {
            let memory = MemorySpaceRaw::from_bits(raw_bits);
            Bar::MemorySpace {
                r#type: memory.r#type(),
                pre_fetch: memory.prefetchable(),
                addr: memory.addr(),
            }
        }
    }
}

#[bitfield(u32)]
struct BarRaw {
    /// true if the bar is has an io space layout otherwise has a memory space layout
    #[bits(1)]
    io_space: bool,
    #[bits(31)]
    _rest: u32,
}

#[bitfield(u32)]
struct MemorySpaceRaw {
    /// true if the bar is has an io space layout otherwise has a memory space layout
    /// thus this field must always be false
    #[bits(1)]
    io_space: bool,
    #[bits(2)]
    r#type: u8,
    #[bits(1)]
    prefetchable: bool,
    #[bits(28)]
    addr: u32,
}

#[bitfield(u32)]
struct IoSpaceRaw {
    /// true if the bar is has an io space layout otherwise has a memory space layout
    /// thus this field must always be true
    #[bits(1)]
    io_space: bool,
    #[bits(1)]
    _reserved: u8,
    #[bits(30)]
    addr: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum MemorySpaceType {
    Wide32 = 0x0,
    Wide64 = 0x2,
}

#[derive(Debug, Clone, Copy)]
pub enum Bar {
    MemorySpace {
        r#type: u8,
        pre_fetch: bool,
        addr: u32,
    },
    IoSpace {
        addr: u32,
    },
}

#[bitfield(u8)]
pub struct HeaderTypeInfo {
    #[bits(7)]
    pub header_type: HeaderType,
    #[bits(1)]
    pub multi_func: bool,
}

#[repr(u8)]
#[bitenum(all = false)]
#[derive(Debug, Copy, Clone)]
pub enum HeaderType {
    GeneralDevice = 0x0,
    PciTopci = 0x1,
    PciTocardbus = 0x2,
    #[fallback]
    Invalid,
}

#[bitfield(u16)]
pub struct CommandReg {
    #[bits(1)]
    io_space: bool,
    #[bits(1)]
    mem_space: bool,
    #[bits(1)]
    bus_master: bool,
    #[bits(1)]
    sepcial_cylces: bool,
    #[bits(1)]
    mem_write_enabled: bool,
    #[bits(1)]
    vga_pallet: bool,
    #[bits(1)]
    parity_error: bool,
    #[bits(1)]
    _reserved: (),
    #[bits(1)]
    serr_enabled: bool,
    #[bits(1)]
    back_to_back_write: bool,
    #[bits(1)]
    interrupt_disable: bool,
    #[bits(5)]
    _reserved2: (),
}

pub fn get_vendor_id(bus: u8, slot: u8) -> Option<u16> {
    let raw_word = unsafe { read_pci_config_reg(bus, slot, 0, 0) };

    let vendor =
        u16::try_from(raw_word & 0xFFFF).expect("vendor id must be a u16 and my math went wrong"); // gets the first half of the u32

    match vendor {
        0xFFFF => None,
        x => Some(x),
    }
}

pub fn get_device_id(bus: u8, slot: u8) -> u16 {
    let raw_word = unsafe { read_pci_config_reg(bus, slot, 0, 0) };

    u16::try_from((raw_word >> 16) & 0xFFFF)
        .expect("Device id must be a u16 and my math went wrong") // gets the secnond half of the u32
}

pub fn get_info(bus: u8, slot: u8, func: u8) -> Option<DeviceInfo> {
    let mut info: [u32; 16] = [0u32; 16];
    info[0] = unsafe { read_pci_config_reg(bus, slot, func, 0) };
    info[1] = unsafe { read_pci_config_reg(bus, slot, func, 0x4) };
    info[2] = unsafe { read_pci_config_reg(bus, slot, func, 0x8) };
    info[3] = unsafe { read_pci_config_reg(bus, slot, func, 0xC) };
    info[4] = unsafe { read_pci_config_reg(bus, slot, func, 0x10) };
    info[5] = unsafe { read_pci_config_reg(bus, slot, func, 0x14) };
    info[6] = unsafe { read_pci_config_reg(bus, slot, func, 0x18) };
    info[7] = unsafe { read_pci_config_reg(bus, slot, func, 0x1C) };
    info[8] = unsafe { read_pci_config_reg(bus, slot, func, 0x20) };
    info[9] = unsafe { read_pci_config_reg(bus, slot, func, 0x24) };
    info[10] = unsafe { read_pci_config_reg(bus, slot, func, 0x28) };
    info[11] = unsafe { read_pci_config_reg(bus, slot, func, 0x2C) };
    info[12] = unsafe { read_pci_config_reg(bus, slot, func, 0x30) };
    info[13] = unsafe { read_pci_config_reg(bus, slot, func, 0x34) };
    info[14] = unsafe { read_pci_config_reg(bus, slot, func, 0x38) };
    info[15] = unsafe { read_pci_config_reg(bus, slot, func, 0x3C) };

    let info = unsafe { core::mem::transmute::<[u32; 16], DeviceInfo>(info) };

    if info.vendor_id == 0xFFFF {
        None
    } else {
        Some(info)
    }
}

unsafe fn read_pci_config_reg(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address: usize = usize::from(bus) << 16
        | usize::from(slot) << 11
        | usize::from(func) << 8
        | usize::from(offset)
        | 0x80000000;

    let mut port = port::PortWriteOnly::<u32>::new(0xCF8);
    unsafe {
        port.write(address.try_into().unwrap());
    }

    let mut port_reader = port::PortReadOnly::<u32>::new(0xCFC);

    unsafe { port_reader.read() }

    // result
    //
    // if result == 0xFFFF {
    //     return 0xFFFF;
    // }
    // (result) & 0xFFFF
    // let result_upper_half = port_result & 0xFFFF;1
    // let result_lower_half = port_result & 0xFFFF;
}

/// enumerates all pci devices
pub fn enumerate() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    for bus in 0u8..=255 {
        for dev in 0u8..32 {
            if let Some(info) = get_info(bus, dev, 0) {
                devices.push(info);

                if info.header_type.multi_func() {
                    for func in 1u8..=7 {
                        if let Some(info) = get_info(bus, dev, func) {
                            devices.push(info);
                        }
                    }
                }
            }
        }
    }

    devices
}

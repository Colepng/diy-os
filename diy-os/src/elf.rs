use x86_64::{PhysAddr, VirtAddr};

/// Implementation based off of <https://uclibc.org/docs/elf-64-gen.pdf> and <https://wiki.osdev.org/ELF>

#[derive(Debug)]
#[repr(C)]
pub struct Header {
    pub identification: Identification,
    pub object_type: ObjectType,
    pub machine: Machine,
    pub version: Version,
    /// Contains the virtual address of the program entry. If there is no entry point, field
    /// contains zero.
    pub entery_address: VirtAddr,
    /// Contains the file offset in bytes of the program header table.
    pub program_header_table_offset: usize,
    /// Contains the file offset in bytes of the section header table.
    pub section_header_table_offset: usize,
    /// Contains processor-specific flags
    pub flags: u32,
    /// Contains the size of the header in bytes
    pub header_size: u16,
    /// Contains the size of a program header entry
    pub program_header_entry_size: u16,
    /// Contains the number of entries in the program header table
    pub program_header_entry_count: u16,
    /// Contains the size of a section header entry
    pub section_header_entry_size: u16,
    /// Contains the number of entries in the section header table
    pub section_header_entry_count: u16,
    /// Contains the section header table index of the section containing the section name string
    /// table. If there is no section name string table, this field has the value `SHN_UNDEF`.
    pub section_header_section_name_index: u16,
}

#[derive(Debug)]
#[repr(C)]
pub struct Identification {
    /// contains a magic number to identify the file as an ELF object file. Contains the ascii
    /// characters '\x7f', 'E', 'L', and 'F', respectively.
    magic_number: [u8; 4],
    class: Class,
    endian: Endian,
    /// Identifies the version of the object file format. Should be have a value of 1
    elf_header_version: u8,
    os_abi: OSAbi,
    abi_version: u8,
    _padding: [u8; 7],
}

#[derive(Debug)]
#[repr(u8)]
/// Identifies the class of an object, or it's capacity.
pub enum Class {
    Bit32Objects = 1,
    Bit64Objects = 2,
}

#[derive(Debug)]
#[repr(u8)]
/// Specifies the data encoding of the object file data structures
pub enum Endian {
    LittleEndian = 1,
    BigEndian = 2,
}

#[derive(Debug)]
#[repr(u8)]
/// Identifies the operating system and ABI for which the object is prepared for. Some fields in
/// other ELF structures have flags and values that have environment-specific meaning.
pub enum OSAbi {
    SysV = 0,
    HPUX = 1,
    Standalone = 255,
}

#[derive(Debug)]
#[repr(u16)]
/// Identifies the object file type.
pub enum ObjectType {
    None,
    Reloctable,
    Executable,
    SharedObject,
    Core,
    /// Environment-specific use
    LoOs = 0xfe00,
    /// Environment-specific use
    HiOs = 0xfeff,
    /// Processor-specific
    LowProc = 0xff00,
    /// Processor-specific
    HighProc = 0xffff,
}

/// Combination of the <https://wiki.osdev.org/ELF> instruction set archtecits table and the
/// `e_machine` table on section 1-4 of <http://www.skyfree.org/linux/references/ELF_Format.pdf>
/// Identifies the target architecture
#[derive(Debug)]
#[repr(u16)]
#[allow(clippy::too_long_first_doc_paragraph)]
pub enum Machine {
    None = 0x00,
    ATAndTWE32100 = 0x01,
    SPARC = 0x02,
    Intel80386 = 0x03,
    Motorola68000 = 0x04,
    Motorola88000 = 0x05,
    Intel80860 = 0x07,
    MIPSRS3000 = 0x08,
    PowerPC = 0x014,
    ARM = 0x28,
    SuperH = 0x2A,
    #[allow(non_camel_case_types)]
    IA_64 = 0x32,
    X86_64 = 0x3E,
    AAarch64 = 0xB7,
    #[allow(non_camel_case_types)]
    RISC_V = 0xF3,
}

/// Identifies the version of the object file format. Should be have a value of 1
#[derive(Debug)]
#[repr(transparent)]
pub struct Version(pub u32);

#[derive(Debug)]
#[repr(C)]
pub struct ProgramHeaderTableEntry {
    pub segment_type: SegmentType,
    pub flags: ProgramHeaderFlags,
    pub offset: u64,
    pub virtual_address: VirtAddr,
    /// Reserved for systems using physical addressing
    physical_address: PhysAddr,
    pub size_of_segment_file: u64,
    pub size_of_segment_mem: u64,
    pub alignment: u64,
}
#[derive(Debug)]
#[repr(u32)]
pub enum SegmentType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interpeter = 3,
    Note = 4,
    /// Reserved/unused
    SharedLib = 5,
    ProgramHeaderTable = 6,
    /// Environment-specific use
    LoOs = 0x6000_0000,
    /// Environment-specific use
    HiOs = 0x6fff_ffff,
    /// Processor-specific use
    LoProc = 0x7000_0000,
    /// Processor-specific use
    HiProc = 0x7fff_ffff,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramHeaderFlags(pub u32);

#[derive(Debug)]
#[repr(u32)]
pub enum ProgramHeaderBitFlagsMasks {
    ExecutePermission = 0b1,
    WritePermission = 0b10,
    ReadPermission = 0b100,
    OsSpecific = 0x00ff_0000,
    ProccessoSpecific = 0xff00_0000,
}

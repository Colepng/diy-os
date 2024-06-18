#[derive(Debug)]
#[repr(C)]
pub struct Header {
    identification: Identification,
    pub object_type: ObjectType,
    pub machine: Machine,
    version: Version,
    entery_address: usize,
    program_header_table_offset: usize,
    section_header_table_offset: usize,
    flags: u32,
    header_size: u16,
    program_header_entry_size: u16,
    program_header_entry_count: u16,
    section_header_entry_count: u16,
    section_header_entry_size: u16,
    section_header_section_name_index: u16,
}

#[derive(Debug)]
#[repr(u16)]
pub enum ObjectType {
    None,
    Reloctable,
    SharedObject,
    Core,
    LowProc = 0xff00, // Processor-specific
    HighProc = 0xffff, // Processor-specific
}

/// Combination of the https://wiki.osdev.org/ELF instruction set archtecits table and the
/// e_machine table on section 1-4 of http://www.skyfree.org/linux/references/ELF_Format.pdf
#[derive(Debug)]
#[repr(u16)]
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

#[derive(Debug)]
struct Version(pub u32);

#[derive(Debug)]
struct Identification {
    magic_number: [u8; 4],
    class: Class,
    endian: u8,
    elf_header_version: u8,
    os_abi: u8,
    _padding: [u8; 8],
}

#[derive(Debug)]
#[repr(u8)]
enum Class {
    InvalidClass,
    Bit32Objects,
    Bit64Objects,
}

#[derive(Debug)]
struct ProgramHeader {
}

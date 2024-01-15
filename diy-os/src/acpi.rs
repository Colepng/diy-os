pub mod system_description_tables {
    use core::mem::{self, size_of};

    /// The header on all System Description Tables
    #[allow(dead_code)]
    #[repr(C, packed)]
    #[derive(Debug)]
    pub struct SystemDescriptionTableHeader {
        pub signature: [u8; 4],
        pub length: u32,
        pub revision: u8,
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub oem_id_table_id: [u8; 8],
        pub oem_revison: u32,
        pub creator_id: u32,
        pub creator_revision: u32,
    }

    /// eXtended Root System Descriptor Pointer
    #[allow(dead_code)]
    #[repr(C, packed)]
    #[derive(Debug)]
    pub struct RSDP {
        pub signature: [u8; 8],
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub revision: u8,
        pub rsdt_address: u32, // deprecated since version 2.0
        // version 2 only
        pub length: u32,
        pub xsdt_address: u64,
        pub extended_checksum: u8,
        pub reserved: [u8; 3],
    }

    /// eXtended System Descriptor Table
    ///
    /// The XSDT has an array of pointers to System Descriptor Tables but because rust does not
    /// suport support variable arrays it's not represented in the struct.
    ///
    /// The helper method [`XDST::to_slice_of_tables`] returns a slice of mutable references to the
    /// tables.
    #[allow(dead_code)]
    #[repr(C, packed)]
    pub struct XSDT {
        sdt: SystemDescriptionTableHeader,
    }

    impl XSDT {
        ///
        /// # Safety
        /// The caller must make sure that address is a valid physical address pointing to the XDST
        /// and the offset is the correct physical memory offset
        pub unsafe fn new_ref(address: usize, offset: usize) -> &'static mut XSDT {
            unsafe { &mut *core::ptr::from_exposed_addr_mut(address + offset) }
        }

        pub fn to_slice_of_tables(
            &mut self,
            offset: usize,
        ) -> &'static mut [&'static mut SystemDescriptionTableHeader] {
            let ptr_to_self = self as *mut XSDT;
            let ptr_to_tables = unsafe {
                ptr_to_self
                    .add(1)
                    .cast::<*mut SystemDescriptionTableHeader>()
            };

            let ptrs =
                unsafe { core::slice::from_raw_parts_mut(ptr_to_tables, self.number_of_entries()) };

            for ptr in ptrs.iter_mut() {
                unsafe {
                    *ptr = ptr.byte_add(offset);
                }
            }

            unsafe {
                mem::transmute::<
                    &mut [*mut SystemDescriptionTableHeader],
                    &'static mut [&'static mut SystemDescriptionTableHeader],
                >(ptrs)
            }
        }

        pub const fn number_of_entries(&self) -> usize {
            (self.sdt.length as usize - size_of::<Self>()) / 8
        }
    }

    /// Multiple APIC Description Table
    #[allow(dead_code)]
    #[repr(C, packed)]
    struct MADT {
        sdt: SystemDescriptionTableHeader,
        local_apic_address: u32,
        flags: u32,
    }
}

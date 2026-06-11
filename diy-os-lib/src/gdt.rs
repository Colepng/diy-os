use lazy_static::lazy_static;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, Segment};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::{VirtAddr, structures::gdt::SegmentSelector};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        #[allow(static_mut_refs)]
        let tss_selector = gdt.append(Descriptor::tss_segment(unsafe { &TSS }));
        (
            gdt,
            Selectors {
                kernel_code_selector,
                kernel_data_selector,
                user_code_selector,
                user_data_selector,
                tss_selector,
            },
        )
    };
}

const STACK_SIZE: usize = 4096 * 5;
// has to be mut otherwise doesn't get maded in or smth
// TODO: fucking fix this dumbass
// just allocate some pages
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut PRIVILEGE_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub fn init() {
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack_start = VirtAddr::from_ptr(&raw mut STACK);
            stack_start + STACK_SIZE as u64
        };
    }

    unsafe {
        TSS.privilege_stack_table[0] = {
            let stack_start = VirtAddr::from_ptr(&raw mut PRIVILEGE_STACK);
            stack_start + STACK_SIZE as u64
        };
    }

    GDT.0.load();

    unsafe {
        CS::set_reg(GDT.1.kernel_code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

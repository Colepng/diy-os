use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use spinlock::Spinlock;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::{FrameDeallocator, PageTableFlags as Flags};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

pub static PMM: Spinlock<Option<BootInfoFrameAllocator>> = Spinlock::new(None);

pub fn setup_virtual_memory_map(
    falloc: &mut impl FrameAllocator<Size4KiB>,
    offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let flags = Flags::GLOBAL | Flags::PRESENT | Flags::WRITABLE;

    let (p4, frame) = new_table(falloc, offset);

    let (p3, frame) = new_table(falloc, offset);
    let mut entry = PageTableEntry::new();
    entry.set_frame(frame, flags);
    p4[0] = entry;

    let (p2, frame) = new_table(falloc, offset);
    let mut entry = PageTableEntry::new();
    entry.set_frame(frame, flags);
    p3[0] = entry;

    let (p1, frame) = new_table(falloc, offset);
    let mut entry = PageTableEntry::new();
    entry.set_frame(frame, flags);
    p2[0] = entry;

    unsafe { OffsetPageTable::new(p4, offset) }
}

pub fn clone_addresss_space(
    falloc: &mut impl FrameAllocator<Size4KiB>,
    offset: VirtAddr,
    plm4: PageTable,
) -> OffsetPageTable<'static> {
    for entry in plm4.iter() {
        let table = unsafe { core::mem::transmute::<&PageTableEntry, &PageTable>(entry) }; // lv3
        for entry in table.iter() {
            let table = unsafe { core::mem::transmute::<&PageTableEntry, &PageTable>(entry) }; // lv2
            for entry in table.iter() {
                let table = unsafe { core::mem::transmute::<&PageTableEntry, &PageTable>(entry) }
            }
        }
    }

    todo!()
    // let flags = Flags::GLOBAL | Flags::PRESENT | Flags::WRITABLE;
    //
    // let (p4, frame) = new_table(falloc, offset);
    //
    // let (p3, frame) = new_table(falloc, offset);
    // let mut entry = PageTableEntry::new();
    // entry.set_frame(frame, flags);
    // p4[0] = entry;
    //
    // let (p2, frame) = new_table(falloc, offset);
    // let mut entry = PageTableEntry::new();
    // entry.set_frame(frame, flags);
    // p3[0] = entry;
    //
    // let (p1, frame) = new_table(falloc, offset);
    // let mut entry = PageTableEntry::new();
    // entry.set_frame(frame, flags);
    // p2[0] = entry;
    //
    // unsafe { OffsetPageTable::new(p4, offset) }
}

fn new_table(
    falloc: &mut impl FrameAllocator<Size4KiB>,
    offset: VirtAddr,
) -> (&'static mut PageTable, PhysFrame) {
    alloc_table(falloc, offset, PageTable::new())
}

pub fn alloc_table(
    falloc: &mut impl FrameAllocator<Size4KiB>,
    offset: VirtAddr,
    new_table: PageTable,
) -> (&'static mut PageTable, PhysFrame) {
    let table_frame = falloc.allocate_frame().unwrap();
    let table_paddr = table_frame.start_address();
    let table_vaddr = VirtAddr::new(table_paddr.as_u64() + offset.as_u64());
    let table_ptr = table_vaddr.as_mut_ptr::<PageTable>();
    let table = unsafe { table_ptr.as_uninit_mut().unwrap() };
    let table = table.write(new_table);

    (table, table_frame)
}

/// initalis a [`OffsetPageTable`] from a `physical_memory_offset`
///
/// # Safety
/// The caller must guarantee that the `physical_memory_offset` is correct.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);

        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// # Safety
/// The caller must guarantee that the `physical_memory_offset` is correct.
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let physical_start_address = level_4_table_frame.start_address();
    let virt_start_adress = physical_memory_offset + physical_start_address.as_u64();

    let page_table_ptr: *mut PageTable = virt_start_adress.as_mut_ptr();

    // SAFETY: Deference of raw pointer which is correct as long as the `physical_memory_offset` is
    // correct
    unsafe { &mut *page_table_ptr }
}

/// A [`FrameAllocator`] that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a [`FrameAllocator`] from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    #[must_use]
    pub const unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        Self {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        // transform to an iterator of frame start addresses
        // 4096 = 4 KiB = the frame size
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {}
}

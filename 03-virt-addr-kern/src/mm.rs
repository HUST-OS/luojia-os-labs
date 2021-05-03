//! 虚拟内存模块

use alloc::alloc::Layout;
use buddy_system_allocator::LockedHeap;

const KERNEL_HEAP_SIZE: usize = 64 * 1024;

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

// 全局的堆分配器
#[global_allocator]
static HEAP: LockedHeap<32> = LockedHeap::empty();

#[cfg_attr(not(test), alloc_error_handler)]
#[allow(unused)]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("alloc error for layout {:?}", layout)
}

pub(crate) fn heap_init() {
    unsafe {
        HEAP.lock().init(
            HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE
        )
    }
    let mut vec = Vec::new();
    for i in 0..5 {
        vec.push(i);
    }
    println!("[kernel] Alloc test: {:?}", vec);
}

const PAGE_SIZE_BITS: usize = 12; // on RISC-V RV64, 4KB
const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
const PADDR_SPACE_BITS: usize = 56;
const PPN_VALID_MASK: usize = (1 << (PADDR_SPACE_BITS - PAGE_SIZE_BITS)) - 1;
// const VADDR_SPACE_BITS: usize = 39;
// const VPN_VALID_MASK: usize = (1 << (VADDR_SPACE_BITS - PAGE_SIZE_BITS)) - 1;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub fn page_number(&self) -> PhysPageNum { 
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }
    // pub fn page_offset(&self) -> usize { 
    //     self.0 & (PAGE_SIZE - 1)
    // }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct VirtAddr(pub usize);

// impl VirtAddr {
//     pub fn page_number(&self) -> VirtPageNum { 
//         VirtPageNum(self.0 >> PAGE_SIZE_BITS)
//     }
//     pub fn page_offset(&self) -> usize { 
//         self.0 & (PAGE_SIZE - 1)
//     }
// }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PhysPageNum(usize);

impl PhysPageNum {
    pub fn addr_begin(&self) -> PhysAddr {
        PhysAddr(self.0 << PAGE_SIZE_BITS)
    }
    pub fn next_page(&self) -> PhysPageNum {
        PhysPageNum(self.0.wrapping_add(1) & PPN_VALID_MASK)
    }
    pub fn is_within_range(&self, begin: PhysPageNum, end: PhysPageNum) -> bool {
        if begin.0 <= end.0 {
            begin.0 <= self.0 && self.0 < end.0
        } else {
            begin.0 <= self.0 || self.0 < end.0
        }
    }
    pub fn is_aligned_like(&self, layout: FrameLayout) -> bool {
        self.0 % layout.frame_align() == 0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct VirtPageNum(usize);

// impl VirtPageNum {
//     pub fn addr_begin(&self) -> VirtAddr {
//         VirtAddr(self.0 << PAGE_SIZE_BITS)
//     }
// }

use alloc::vec::Vec;

// 页帧分配器。**对于物理空间的一个片段，只存在一个页帧分配器，无论有多少个处理核**
#[derive(Debug)]
pub struct StackFrameAllocator {
    current: PhysPageNum,
    end: PhysPageNum,
    recycled: Vec<PhysPageNum>,
}

impl StackFrameAllocator {
    pub fn new(start: PhysPageNum, end: PhysPageNum) -> Self {
        StackFrameAllocator { current: start, end, recycled: Vec::new() }
    }
    pub fn allocate_frame(&mut self) -> Result<PhysPageNum, FrameAllocError> {
        if let Some(ppn) = self.recycled.pop() {
            Ok(ppn)
        } else {
            if self.current == self.end {
                Err(FrameAllocError)
            } else {
                let ans = self.current;
                self.current = self.current.next_page();
                Ok(ans)
            }
        }
    }
    pub fn deallocate_frame(&mut self, ppn: PhysPageNum) {
        // validity check
        if ppn.is_within_range(self.current, self.end) || self.recycled.iter().find(|&v| {*v == ppn}).is_some() {
            panic!("Frame ppn={:x?} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrameAllocError;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrameLayout {
    // 对齐到的页帧数。比如，如果是1，说明按字节运算，对齐到4K字节，
    // 如果是512，对齐到2M字节；如果是512*512，对齐到1G字节。
    frame_align: usize,
}

// 应当从PageMode::get_layout_for_level中获得
impl FrameLayout {
    // 未检查参数，用于实现PageMode
    pub const unsafe fn new_unchecked(frame_align: usize) -> Self {
        Self { frame_align }
    }
    pub const fn frame_align(&self) -> usize {
        self.frame_align
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrameLayoutError;

pub(crate) fn test_frame_alloc() {
    let from = PhysAddr(0x80_000_000).page_number();
    let to = PhysAddr(0x100_000_000).page_number();
    let mut alloc = StackFrameAllocator::new(from, to);
    let f1 = alloc.allocate_frame();
    assert_eq!(f1, Ok(PhysPageNum(0x80000)), "first allocation");
    let f2 = alloc.allocate_frame();
    assert_eq!(f2, Ok(PhysPageNum(0x80001)), "second allocation");
    alloc.deallocate_frame(f1.unwrap());
    let f3 = alloc.allocate_frame();
    assert_eq!(f3, Ok(PhysPageNum(0x80000)), "after free first, third allocation");
    println!("[kernel-frame-test] Frame allocator test passed");
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AddressSpaceId(u16);

impl AddressSpaceId {
    fn next_asid(&self, max_asid: AddressSpaceId) -> Option<AddressSpaceId> {
        if self.0 >= max_asid.0 {
            None
        } else {
            Some(AddressSpaceId(self.0.wrapping_add(1)))
        }
    }
}

const DEFAULT_ASID: AddressSpaceId = AddressSpaceId(0); // RISC-V架构规定，必须实现

// 每个平台上是不一样的，需要通过读写satp寄存器获得
pub fn max_asid() -> AddressSpaceId {
    #[cfg(target_pointer_width = "64")]
    let mut val: usize = ((1 << 16) - 1) << 44;
    #[cfg(target_pointer_width = "32")]
    let mut val: usize = ((1 << 9) - 1) << 22;
    unsafe { asm!("
        csrr    {tmp}, satp
        or      {val}, {tmp}, {val}
        csrw    satp, {val}
        csrrw   {val}, satp, {tmp}
    ", tmp = out(reg) _, val = inlateout(reg) val) };
    #[cfg(target_pointer_width = "64")]
    return AddressSpaceId(((val >> 44) & ((1 << 16) - 1)) as u16);
    #[cfg(target_pointer_width = "32")]
    return AddressSpaceId(((val >> 22) & ((1 << 9) - 1)) as u16);
}

// 在看代码的同志们可能发现，这里分配地址空间编号的算法和StackFrameAllocator很像。
// 这里需要注意的是，分配页帧的算法经常要被使用，而且包含很多参数，最好最快的写法不一定是简单的栈式回收分配，
// 更好的高性能内核设计，页帧分配的算法或许会有较大的优化空间。
// 可以包含的参数，比如，页帧的内存布局，包括内存对齐的选项，这是大页优化非常需要的选项。
// 但是地址空间编号的分配算法而且不需要经常调用，所以可以设计得很简单，普通栈式回收的算法就足够使用了。

// 地址空间编号分配器，**每个处理核都有一个**
#[derive(Debug)]
pub struct StackAsidAllocator {
    current: AddressSpaceId,
    exhausted: bool, 
    max: AddressSpaceId,
    recycled: Vec<AddressSpaceId>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct AsidAllocError;

impl StackAsidAllocator {
    pub fn new(max_asid: AddressSpaceId) -> Self {
        StackAsidAllocator { current: DEFAULT_ASID, exhausted: false, max: max_asid, recycled: Vec::new() }
    }

    pub fn allocate_asid(&mut self) -> Result<AddressSpaceId, AsidAllocError> {
        if let Some(asid) = self.recycled.pop() {
            return Ok(asid)
        }
        if self.exhausted {
            return Err(AsidAllocError)
        }
        if self.current == self.max {
            self.exhausted = true;
            return Ok(self.max)
        }
        if let Some(next) = self.current.next_asid(self.max) {
            let ans = self.current;
            self.current = next;
            Ok(ans)
        } else {
            Err(AsidAllocError)
        }
    }
    
    fn deallocate_asid(&mut self, asid: AddressSpaceId) {
        if asid.next_asid(self.max).is_none() || self.recycled.iter().find(|&v| {*v == asid}).is_some() {
            panic!("Asid {:x?} has not been allocated!", asid);
        }
        self.recycled.push(asid);
    }
}

pub(crate) fn test_asid_alloc() {
    let max_asid = AddressSpaceId(0xffff);
    let mut alloc = StackAsidAllocator::new(max_asid);
    let a1 = alloc.allocate_asid();
    assert_eq!(a1, Ok(AddressSpaceId(0)), "first allocation");
    let a2 = alloc.allocate_asid();
    assert_eq!(a2, Ok(AddressSpaceId(1)), "second allocation");
    alloc.deallocate_asid(a1.unwrap());
    let a3 = alloc.allocate_asid();
    assert_eq!(a3, Ok(AddressSpaceId(0)), "after free first one, third allocation");
    for _ in 0..max_asid.0 - 2 {
        alloc.allocate_asid().unwrap();
    }
    let an = alloc.allocate_asid();
    assert_eq!(an, Ok(max_asid), "last asid");
    let an = alloc.allocate_asid();
    assert_eq!(an, Err(AsidAllocError), "when asid exhausted, allocate next");
    alloc.deallocate_asid(a2.unwrap());
    let an = alloc.allocate_asid();
    assert_eq!(an, Ok(AddressSpaceId(1)), "after free second one, allocate next");
    let an = alloc.allocate_asid();
    assert_eq!(an, Err(AsidAllocError), "no asid remains, allocate next");
    
    let mut alloc = StackAsidAllocator::new(DEFAULT_ASID); // asid not implemented
    let a1 = alloc.allocate_asid();
    assert_eq!(a1, Ok(AddressSpaceId(0)), "asid not implemented, first allocation");
    let a2 = alloc.allocate_asid();
    assert_eq!(a2, Err(AsidAllocError), "asid not implemented, second allocation");

    println!("[kernel-asid-test] Asid allocator test passed");
}

pub trait FrameAllocator {
    fn allocate_frame(&self) -> Result<PhysPageNum, FrameAllocError>;
    fn deallocate_frame(&self, ppn: PhysPageNum);
}

pub type DefaultFrameAllocator = spin::Mutex<StackFrameAllocator>;

impl FrameAllocator for DefaultFrameAllocator {
    fn allocate_frame(&self) -> Result<PhysPageNum, FrameAllocError> {
        self.lock().allocate_frame()
    }
    fn deallocate_frame(&self, ppn: PhysPageNum) {
        self.lock().deallocate_frame(ppn)
    }
}

impl<A: FrameAllocator + ?Sized> FrameAllocator for &A { 
    fn allocate_frame(&self) -> Result<PhysPageNum, FrameAllocError> {
        (**self).allocate_frame()
    }
    fn deallocate_frame(&self, ppn: PhysPageNum) {
        (**self).deallocate_frame(ppn)
    }
}

// 表示整个页帧内存的所有权
#[derive(Debug)]
pub struct FrameBox<A: FrameAllocator = DefaultFrameAllocator> {
    ppn: PhysPageNum, // 相当于*mut类型的指针
    frame_alloc: A,
}

impl<A: FrameAllocator> FrameBox<A> {
    // 分配页帧并创建FrameBox
    pub fn try_new_in(frame_alloc: A) -> Result<FrameBox<A>, FrameAllocError> {
        let ppn = frame_alloc.allocate_frame()?;
        Ok(FrameBox { ppn, frame_alloc })
    }
    // unsafe说明。调用者必须保证以下约定：
    // 1. ppn只被一个FrameBox拥有，也就是不能破坏所有权约定
    // 2. 这个ppn是由frame_alloc分配的
    unsafe fn from_ppn(ppn: PhysPageNum, frame_alloc: A) -> Self {
        Self { ppn, frame_alloc }
    }

    fn phys_page_num(&self) -> PhysPageNum {
        self.ppn
    }
}

impl<A: FrameAllocator> Drop for FrameBox<A> {
    fn drop(&mut self) {
        // 释放所占有的页帧
        self.frame_alloc.deallocate_frame(self.ppn);
    }
}

// 没有实现drop函数

// Sv39分页系统模式；RISC-V RV64下有效
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Sv39;

// 分页模式
//
// 在每个页式管理模式下，我们认为分页系统分为不同的等级，每一级如果存在大页页表，都应当有相应的对齐要求。
// 然后当前的页式管理模式，一定有一个固定的最大等级。
//
// 如果虚拟内存的模式是直接映射或者线性映射，这将不属于分页模式的范围。应当混合使用其它的地址空间，综合成为更大的地址空间。
pub trait PageMode: Copy {
    // 得到这一层大页物理地址最低的对齐要求
    fn get_layout_for_level(level: PageLevel) -> FrameLayout;
    // 得到根页表的等级。按如下方法计算：如果虚拟地址包含vpn[n]、vpn[n-1]...vpn[0]，那么根页表等级为n+1。
    fn root_level() -> PageLevel;
    // 得到从高到低的页表等级
    fn visit_levels() -> &'static [PageLevel];
    // 得到一个虚拟页号各个等级的索引，从高到低
    fn vpn_index(vpn: VirtPageNum, level: PageLevel) -> usize;
    // 页式管理模式的页表项类型
    type ModeEntry;
    // 解释页表项目；如果项目无效，返回None，可以直接操作pte写入其它数据
    unsafe fn convert_entry_mut(pte: &mut PageTableEntry) -> Option<&mut Self::ModeEntry>;
    // 创建页表时，把它的所有条目设置为无效条目
    unsafe fn fill_page_table_invalid(table: &mut PageTable);
    // 页表项的设置
    type Flags;
    // 写数据到页表项目
    fn write_ppn_flags(entry: &mut Self::ModeEntry, ppn: PhysPageNum, flags: Self::Flags);
}

// 我们认为今天的分页系统都是分为不同的等级，就是多级页表，这里表示页表的等级是多少
// todo: 实现一些函数，用于分页算法
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PageLevel(u8); 

impl PageMode for Sv39 {
    fn get_layout_for_level(level: PageLevel) -> FrameLayout {
        unsafe { match level.0 {
            0 => FrameLayout::new_unchecked(1), // 4K页，最低层页
            1 => FrameLayout::new_unchecked(512), // 2M页
            2 => FrameLayout::new_unchecked(512 * 512), // 1G页，最高层大页
            _ => unimplemented!("this level does not exist on Sv39")
        } }
    }
    fn root_level() -> PageLevel {
        PageLevel(3)
    }
    fn visit_levels() -> &'static [PageLevel] {
        &[PageLevel(2), PageLevel(1), PageLevel(0)]
    }
    fn vpn_index(vpn: VirtPageNum, level: PageLevel) -> usize {
        (vpn.0 >> (level.0 * 9)) & 511
    }
    type ModeEntry = Sv39PageEntry;
    unsafe fn convert_entry_mut(pte: &mut PageTableEntry) -> Option<&mut Sv39PageEntry> {
        let ans = unsafe { &mut *(&mut pte.child_page as *mut _ as *mut Sv39PageEntry) };
        if ans.flags().contains(Sv39Flags::V) {
            Some(ans)
        } else {
            None
        }
    }
    unsafe fn fill_page_table_invalid(table: &mut PageTable) {
        table.entries = unsafe { core::mem::MaybeUninit::zeroed().assume_init() }; // 全零
    }
    type Flags = Sv39Flags;
    fn write_ppn_flags(entry: &mut Sv39PageEntry, ppn: PhysPageNum, flags: Self::Flags) {
        entry.write_ppn_flags(ppn, flags);
    }
}

#[repr(C)]
pub struct Sv39PageEntry {
    bits: usize,
}

use bit_field::BitField;

impl Sv39PageEntry {
    #[inline]
    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum(self.bits.get_bits(8..54))
    }
    #[inline]
    pub fn flags(&self) -> Sv39Flags {
        Sv39Flags::from_bits_truncate(self.bits.get_bits(0..8) as u8)
    }
    #[inline]
    pub fn write_ppn_flags(&mut self, ppn: PhysPageNum, flags: Sv39Flags) {
        self.bits = (ppn.0 << 8) | flags.bits() as usize
    }
}

// 表示一个分页系统实现的地址空间
//
// 如果属于直接映射或者线性偏移映射，不应当使用这个结构体，应当使用其它的结构体。
#[derive(Debug)]
pub struct PagedAddrSpace<M: PageMode, A: FrameAllocator = DefaultFrameAllocator> {
    root_frame: FrameBox<A>,
    frames: Vec<FrameBox<A>>,
    frame_alloc: A,
    page_mode: M,
}

impl<M: PageMode, A: FrameAllocator + Clone> PagedAddrSpace<M, A> {
    // 创建一个空的分页地址空间
    pub fn try_new_in(page_mode: M, frame_alloc: A) -> Result<Self, FrameAllocError> {
        // 新建一个满足根页表对齐要求的帧；虽然代码没有体现，通常对齐要求是1
        let mut root_frame = FrameBox::try_new_in(frame_alloc.clone())?;
        // 向帧里填入一个空的根页表
        unsafe { fill_frame_with_all_invalid_page_table(&mut root_frame, page_mode) };
        Ok(Self { root_frame, frames: Vec::new(), frame_alloc, page_mode })
    }
}

bitflags::bitflags! {
    pub struct Sv39Flags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[inline] unsafe fn unref_ppn_mut<'a>(ppn: PhysPageNum) -> &'a mut PageTable {
    let pa = ppn.addr_begin();
    &mut *(pa.0 as *mut PageTable)
}

#[inline] unsafe fn fill_frame_with_all_invalid_page_table<A: FrameAllocator, M: PageMode>(b: &mut FrameBox<A>, mode: M) {
    let a = &mut *(b.ppn.addr_begin().0 as *mut PageTable);
    M::fill_page_table_invalid(a);
}

impl<M: PageMode, A: FrameAllocator> PagedAddrSpace<M, A> {
    // unsafe fn entry_mut(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
    //     let mut ppn = self.root_frame.phys_page_num();
    //     for level in M::visit_levels() {
    //         let mut page_table = M::unref_ppn_mut(ppn);
    //         let vidx = M::vpn_index(vpn, level);
    //         if let Some(pte) = M::convert_entry_mut(&mut page_table.entries[vidx]) {
    //             ppn = pte.ppn();
    //         } else {
    //             return VacantEntry;
    //         }
    //     }
    //     return Found(ppn)
    // }
    // pub fn allocate_map(&mut self, vpn: VirtPageNum, flags: PageFlags) -> Result<(), FrameAllocError> {
    //     // for level in M::levels().?? {

    //     // }
    //     // 页分配算法，巨难写……留坑
    //     todo!()
    // }
    // pub fn unmap(&mut self, vpn: VirtPageNum) {
    //     todo!()
    // }
}

#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_SIZE / core::mem::size_of::<PageTableEntry>()],
}

#[repr(C)]
pub union PageTableEntry {
    child_page: usize,
    unused_data: usize,
}

// 切换地址空间，同时需要提供1.地址空间的详细设置 2.地址空间编号
// 不一定最后的API就是这样的，留个坑
// pub fn activate_paged(addr_space: &PagedAddrSpace, asid: AddressSpaceId) {
//     todo!()    
// }

// 自身映射地址空间；虚拟地址等于物理地址
//
// 启动这种映射，不需要激活地址空间。
// pub fn activate_identical() { todo!() }

// // 
// // input: v: VirtPageNum, p: PhysPageNum, n: usize, a: PageMode;
// if (v - p) % (a[2].frame_align()) == 0 && n >= a[2].frame_align() {
//     let l2n = (vs2 - ve2) / a[2].frame_align();
//     map(2, ve2, vs2, ve2-v+p);
//     let l1n = (ve2 - ve1 + vs1 - vs2) / a[1].frame_align();
//     map(1, ve1, ve2, ve1-v+p); map(1, vs2, vs1, vs2-v+p);
//     let l0n = (n + ve1 - vs1) / a[0].frame_align();
//     map(0, v, ve1, p); map(0, vs1, v+n, vs1-v+p);
// } else if (v - p) % (a[1].frame_align()) == 0 && n >= a[1].frame_align() {
//     let l1n = (vs1 - ve1) / a[1].frame_align();
//     map(1, ve1, vs1, ve1-v+p);
//     let l0n = (n + ve1 - vs1) / a[0].frame_align();
//     map(0, v, ve1, p); map(0, vs1, v+n, vs1-v+p);
// } else if (v - p) % (a[0].frame_align()) == 0 && n >= a[0].frame_align() {
//     let l0n = n / a[0].frame_align();
//     map(0, v, v+n, p);
// } else {
//     panic!("Can't map v to p under this page mode")
// }

// for level in 0..M::root_level().rev() { // [2, 1, 0]
//     let align = M::get_layout_for_level(level).frame_align();
//     if (v - p) % align != 0 || n < align {
//         continue;
//     }
//     let page_table_ppn = self.frame_alloc.allocate_frame(layout);
//     if !page_table_ppn.is_aligned_like(layout) {
//         continue;
//     }
//     self. // map(...)
//     break;
//     //
// } 

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
// const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
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
    // pub fn addr_begin(&self) -> PhysAddr {
    //     PhysAddr(self.0 << PAGE_SIZE_BITS)
    // }
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
// 这里需要注意的是，分配页帧的算法经常要被使用，最好最快的写法不一定是简单的栈式回收分配，
// 更好的高性能内核设计，页帧分配的算法或许会有较大的优化空间。
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

// 表示整个页帧内存的所有权
struct FrameBox<A: FrameAllocator = DefaultFrameAllocator> {
    ppn: PhysPageNum,
    frame_alloc: A,
}

impl<A: FrameAllocator> FrameBox<A> {
    // unsafe说明。调用者必须保证以下约定：
    // 1. ppn只被一个FrameBox拥有，也就是不能破坏所有权约定
    // 2. 这个ppn是由frame_alloc分配的
    unsafe fn from_ppn(ppn: PhysPageNum, frame_alloc: A) -> Self {
        Self { ppn, frame_alloc }
    }

    fn try_new_in(mut frame_alloc: A) -> Result<Self, FrameAllocError> {
        let ppn = frame_alloc.allocate_frame()?;
        Ok(Self { ppn, frame_alloc })
    }

    fn phys_page_num(&self) -> PhysPageNum {
        self.ppn
    }
}

impl<A: FrameAllocator> Drop for FrameBox<A> {
    fn drop(&mut self) {
        self.frame_alloc.deallocate_frame(self.ppn);
    }
}

// 表示一个分页系统实现的地址空间
pub struct PagedAddrSpace<A: FrameAllocator = DefaultFrameAllocator> {
    root_frame: FrameBox<A>,
    frames: Vec<FrameBox<A>>,
}

impl<A: FrameAllocator> PagedAddrSpace<A> {
    // 创建一个空的分页地址空间
    pub fn try_new_in(mut frame_alloc: A) -> Result<Self, FrameAllocError> {
        let root_frame = FrameBox::try_new_in(frame_alloc)?;
        Ok(Self { root_frame, frames: Vec::new() })
    }
}

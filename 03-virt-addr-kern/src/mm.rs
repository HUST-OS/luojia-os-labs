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
    let f1 = alloc.allocate_frame().unwrap();
    println!("[kernel-frame-test] First alloc: {:x?}", f1);
    let f2 = alloc.allocate_frame().unwrap();
    println!("[kernel-frame-test] Second alloc: {:x?}", f2);
    alloc.deallocate_frame(f1);
    println!("[kernel-frame-test] Free first one");
    let f3 = alloc.allocate_frame().unwrap();
    println!("[kernel-frame-test] Third alloc: {:x?}", f3);
}

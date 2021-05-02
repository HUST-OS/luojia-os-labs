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
    use alloc::vec::Vec;
    let mut vec = Vec::new();
    for i in 0..5 {
        vec.push(i);
    }
    println!("[kernel] Alloc test: {:?}", vec);
}




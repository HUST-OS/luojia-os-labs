use alloc::alloc::Layout;
use buddy_system_allocator::LockedHeap;

const HEAP_SIZE: usize = 16 * 1024;

static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

// 全局的堆分配器
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[cfg_attr(not(test), alloc_error_handler)]
#[allow(unused)]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("alloc error for layout {:?}", layout)
}

pub fn init_heap() {
    unsafe {
        HEAP.lock().init(
            HEAP_SPACE.as_ptr() as usize, HEAP_SIZE
        )
    }
}

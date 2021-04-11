#![no_std]
#![feature(asm)]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
#[doc(hidden)]
pub mod console;
mod syscall;
mod heap;
mod task;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message().unwrap().as_str();
    if let Some(location) = panic_info.location() {
        syscall::sys_panic(Some(location.file()), location.line(), location.column(), err);
    } else {
        syscall::sys_panic(None, 0, 0, err);
    }
    loop {}
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    extern "C" {
        fn sbss(); fn ebss();
    } 
    unsafe { r0::zero_bss(&mut sbss as *mut _ as *mut u64, &mut ebss as *mut _ as *mut u64) };
    heap::init_heap();
    use alloc::sync::Arc;
    use spin::Mutex;
    let exit_code = Arc::new(Mutex::new(0));
    let exit_code_2 = exit_code.clone();
    let main_task = task::UserTask::new(async move {
        *exit_code_2.lock() = main().await;
    });
    let mut scheduler = task::RoundRobinScheduler::new();
    scheduler.push_task(Arc::new(main_task));
    loop {
        if let Some(task) = scheduler.pop_task() {
            task.mark_sleeping();
            let waker = woke::waker_ref(&task);
            let mut context = core::task::Context::from_waker(&*waker);
            let ret = task.future.lock().as_mut().poll(&mut context);
            if let core::task::Poll::Pending = ret {
                task.mark_ready();
                scheduler.push_task(task);
            } // else drop(task);
        } else {
            break
        }
    };
    syscall::sys_exit(*exit_code.lock());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
async fn main() -> i32 {
    panic!("Cannot find main!");
}

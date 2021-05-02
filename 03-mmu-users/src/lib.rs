#![no_std]
#![feature(asm)]
#![feature(linkage)]
#![feature(panic_info_message)]

#[macro_use]
#[doc(hidden)]
pub mod console;
mod syscall;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message().unwrap().as_str();
    if let Some(location) = panic_info.location() {
        sys_panic(Some(location.file()), location.line(), location.column(), err);
    } else {
        sys_panic(None, 0, 0, err);
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
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> SyscallResult { sys_write(fd, buf) }
pub fn exit(exit_code: i32) -> SyscallResult { sys_exit(exit_code) }

#![no_std]
#![feature(asm)]
#![feature(linkage)]
#![feature(panic_info_message)]

#[macro_use]
pub mod console {
    use core::fmt::{self, Write};
    use super::write;
    
    struct Stdout;
    
    const STDOUT: usize = 1;
    
    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            write(STDOUT, s.as_bytes());
            Ok(())
        }
    }
    
    pub fn print(args: fmt::Arguments) {
        Stdout.write_fmt(args).unwrap();
    }
    
    #[macro_export]
    macro_rules! print {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!($fmt $(, $($arg)+)?));
        }
    }
    
    #[macro_export]
    macro_rules! println {
        ($fmt: literal $(, $($arg: tt)+)?) => {
            $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        }
    }
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message().unwrap();
    if let Some(location) = panic_info.location() {
        println!("Panicked at {}:{}, {}", location.file(), location.line(), err);
    } else {
        println!("Panicked: {}", err);
    }
    sys_panic();
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

mod syscall {
    const MODULE_PROCESS: usize = 0x114514;
    const FUNCTION_PROCESS_EXIT: usize = 0x1919810;
    const FUNCTION_PROCESS_PANIC: usize = 0x11451419;

    const MODULE_TEST_INTERFACE: usize = 0x233666;
    const FUNCTION_TEST_WRITE: usize = 0x666233;

    pub struct SyscallResult {
        pub code: usize,
        pub extra: usize,
    }

    fn syscall(module: usize, function: usize, args: [usize; 3]) -> SyscallResult {
        match () {
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            () => {
                let (code, extra);
                unsafe { asm!(
                    "ecall", 
                    in("a0") args[0], in("a1") args[1], in("a2") args[2],
                    in("a6") function, in("a7") module,
                    lateout("a0") code, lateout("a1") extra,
                ) };
                SyscallResult { code, extra }
            },
            #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
            () => {
                drop((module, function, args));
                unimplemented!("not RISC-V instruction set architecture")
            }
        }
    }

    pub fn sys_write(fd: usize, buffer: &[u8]) -> SyscallResult {
        syscall(MODULE_TEST_INTERFACE, FUNCTION_TEST_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
    }

    pub fn sys_exit(exit_code: i32) -> SyscallResult {
        syscall(MODULE_PROCESS, FUNCTION_PROCESS_EXIT, [exit_code as usize, 0, 0])
    }

    pub fn sys_panic() -> SyscallResult {
        syscall(MODULE_PROCESS, FUNCTION_PROCESS_PANIC, [0, 0, 0])
    }
}

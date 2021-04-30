#![feature(naked_functions, asm, global_asm)]
#![feature(panic_info_message)]
#![no_std]
#![no_main]

#[macro_use]
mod console;
mod sbi;
mod app;
mod syscall;
mod executor;

use core::panic::PanicInfo;

pub extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    extern "C" {
        fn stext(); fn etext(); fn srodata(); fn erodata();
        fn sdata(); fn edata(); fn sbss(); fn ebss();
    }
    unsafe { r0::zero_bss(&mut sbss as *mut _ as *mut u64, &mut ebss as *mut _ as *mut u64) };
    println!("Hart id = {}, DTB physical address = {:#x}", hartid, dtb_pa);
    println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    println!(".bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    executor::init();
    app::APP_MANAGER.print_app_info();
    let mut rt = executor::Runtime::new_user();
    rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
    loop {
        use executor::ResumeResult;
        use crate::syscall::{syscall, SyscallOperation};
        match rt.resume() {
            ResumeResult::Syscall(ctx) => {
                match syscall(ctx.a7, ctx.a6, [ctx.a0, ctx.a1, ctx.a2, ctx.a3, ctx.a4, ctx.a5]) {
                    SyscallOperation::Return(ans) => {
                        ctx.a0 = ans.code;
                        ctx.a1 = ans.extra;
                        ctx.sepc = ctx.sepc.wrapping_add(4);
                    }
                    SyscallOperation::Terminate(code) => {
                        println!("[Kernel] Process returned with code {}", code);
                        rt.reset();
                        rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
                    }
                    SyscallOperation::UserPanic(file, line, col, msg) => {
                        let file = file.unwrap_or("<no file>");
                        let msg = msg.unwrap_or("<no message>");
                        println!("[Kernel] User process panicked at '{}', {}:{}:{}", msg, file, line, col);
                        rt.reset();
                        rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
                    }
                }
            },
            ResumeResult::LoadAccessFault(a) => {
                println!("[kernel] Load access fault in application address {:x}, core dumped.", a);
                rt.reset();
                rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
            },
            ResumeResult::StoreAccessFault(a) => {
                println!("[kernel] Store access fault in application address {:x}, core dumped.", a);
                rt.reset();
                rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
            },
            ResumeResult::IllegalInstruction(a) => {
                println!("[kernel] Illegal instruction in application address {:x}, core dumped.", a);
                rt.reset();
                rt.context_mut().sepc = app::APP_MANAGER.prepare_next_app();
            },
            // _ => todo!("handle more exceptions")
        }
    }
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Kernel panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Kernel panicked: {}", info.message().unwrap());
    }
    sbi::shutdown()
}

const BOOT_STACK_SIZE: usize = 4096 * 4 * 8;

#[link_section = ".bss.stack"]
static mut BOOT_STACK: [u8; BOOT_STACK_SIZE] = [0; BOOT_STACK_SIZE];

#[naked]
#[link_section = ".text.entry"] 
#[export_name = "_start"]
unsafe extern "C" fn entry() -> ! {
    asm!("
    # 1. set sp
    # sp = bootstack + (hartid + 1) * 0x10000
    add     t0, a0, 1
    slli    t0, t0, 14
1:  auipc   sp, %pcrel_hi({boot_stack})
    addi    sp, sp, %pcrel_lo(1b)
    add     sp, sp, t0

    # 2. jump to rust_main (absolute address)
1:  auipc   t0, %pcrel_hi({rust_main})
    addi    t0, t0, %pcrel_lo(1b)
    jr      t0
    ", 
    boot_stack = sym BOOT_STACK, 
    rust_main = sym rust_main,
    options(noreturn))
}

#[cfg(not(test))]
global_asm!(include_str!("link_app.S"));

#![feature(naked_functions, asm, global_asm)]
#![feature(panic_info_message)]
#![no_std]
#![no_main]

#[macro_use]
mod console;
mod sbi;

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
    println!("Shutdown machine!");
    sbi::shutdown()
}

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Panicked: {}", info.message().unwrap());
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

global_asm!(include_str!("link_app.S"));

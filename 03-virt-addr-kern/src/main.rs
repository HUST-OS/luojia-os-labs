#![feature(naked_functions, asm, global_asm)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(generator_trait)]
#![feature(destructuring_assignment)]
#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
mod console;
mod sbi;
mod app;
mod syscall;
mod executor;
mod mm;

use core::panic::PanicInfo;
use executor::KernelTrap;
use crate::syscall::{syscall, SyscallOperation};
use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

pub extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    extern "C" { fn sbss(); fn ebss();/* fn ekernel(); */}
    unsafe { r0::zero_bss(&mut sbss as *mut _ as *mut u64, &mut ebss as *mut _ as *mut u64) };
    println!("[kernel] Hart id = {}, DTB physical address = {:#x}", hartid, dtb_pa);
    mm::heap_init();
    mm::test_frame_alloc();
    // 页帧分配器。对整个物理的地址空间来说，无论有多少个核，页帧分配器只有一个。
    let from = mm::PhysAddr(0x80420000).page_number::<mm::Sv39>();
    let to = mm::PhysAddr(0x80800000).page_number::<mm::Sv39>(); // 暂时对qemu写死
    let frame_alloc = spin::Mutex::new(mm::StackFrameAllocator::new(from, to));
    // println!("[kernel-frame] Frame allocator: {:x?}", frame_alloc);
    let mut kernel_addr_space = mm::PagedAddrSpace::try_new_in(mm::Sv39, &frame_alloc)
        .expect("allocate page to create kernel paged address space");
    // println!("[kernel] Kernel address space: {:x?}", kernel_addr_space);
    mm::test_map_solve();
    kernel_addr_space.allocate_map(
        mm::VirtAddr(0x80000000).page_number::<mm::Sv39>(), 
        mm::PhysAddr(0x80000000).page_number::<mm::Sv39>(), 
        1024,
        mm::Sv39Flags::R | mm::Sv39Flags::W | mm::Sv39Flags::X
    ).expect("allocate one mapped space");
    kernel_addr_space.allocate_map(
        mm::VirtAddr(0x80400000).page_number::<mm::Sv39>(), 
        mm::PhysAddr(0x80400000).page_number::<mm::Sv39>(), 
        32,
        mm::Sv39Flags::R | mm::Sv39Flags::W | mm::Sv39Flags::X | mm::Sv39Flags::U
    ).expect("allocate one mapped space");
    // println!("[kernel] Kernel address space: {:x?}", kernel_addr_space);
    mm::test_asid_alloc();
    let max_asid = mm::max_asid();
    let mut asid_alloc = mm::StackAsidAllocator::new(max_asid);
    // println!("[kernel-asid] Asid allocator: {:x?}", asid_alloc);
    let kernel_asid = asid_alloc.allocate_asid().expect("alloc kernel asid");
    use crate::mm::FrameAllocator;
    let user_stack_ppn = frame_alloc.allocate_frame().expect("Alloc user stack");
    println!("User stack ppn: {:?}", user_stack_ppn);
    kernel_addr_space.allocate_map(
        mm::VirtAddr(user_stack_ppn.addr_begin::<mm::Sv39>().0).page_number::<mm::Sv39>(), 
        user_stack_ppn, 
        1,
        mm::Sv39Flags::R | mm::Sv39Flags::W | mm::Sv39Flags::X | mm::Sv39Flags::U
    ).expect("allocate one mapped space");
    unsafe {
        mm::activate_paged_riscv_sv39(kernel_addr_space.root_page_number(), kernel_asid);
    }
    unsafe { riscv::register::sstatus::set_sum() };
    executor::init();
    execute(user_stack_ppn.addr_begin::<mm::Sv39>().0 + 0x1000);
}

fn execute(user_stack: usize) -> ! {
    app::APP_MANAGER.print_app_info();
    let mut rt = executor::Runtime::new_user(app::APP_MANAGER.prepare_next_app(), user_stack);
    loop {
        match Pin::new(&mut rt).resume(()) {
            GeneratorState::Yielded(KernelTrap::Syscall()) => {
                let ctx = rt.context_mut();
                match syscall(ctx.a7, ctx.a6, [ctx.a0, ctx.a1, ctx.a2, ctx.a3, ctx.a4, ctx.a5]) {
                    SyscallOperation::Return(ans) => {
                        ctx.a0 = ans.code;
                        ctx.a1 = ans.extra;
                        ctx.sepc = ctx.sepc.wrapping_add(4);
                    }
                    SyscallOperation::Terminate(code) => {
                        println!("[Kernel] Process returned with code {}", code);
                        rt.prepare_next_app(app::APP_MANAGER.prepare_next_app());
                    }
                    SyscallOperation::UserPanic(file, line, col, msg) => {
                        let file = file.unwrap_or("<no file>");
                        let msg = msg.unwrap_or("<no message>");
                        println!("[Kernel] User process panicked at '{}', {}:{}:{}", msg, file, line, col);
                        rt.prepare_next_app(app::APP_MANAGER.prepare_next_app());
                    }
                }
            },
            GeneratorState::Yielded(KernelTrap::LoadAccessFault(a)) => {
                let ctx = rt.context_mut();
                println!("[kernel] Load access fault to {:#x} in {:#x}, core dumped.", a, ctx.sepc);
                rt.prepare_next_app(app::APP_MANAGER.prepare_next_app());
            },
            GeneratorState::Yielded(KernelTrap::StoreAccessFault(a)) => {
                let ctx = rt.context_mut();
                println!("[kernel] Store access fault to {:#x} in {:#x}, core dumped.", a, ctx.sepc);
                rt.prepare_next_app(app::APP_MANAGER.prepare_next_app());
            },
            GeneratorState::Yielded(KernelTrap::IllegalInstruction(a)) => {
                let ctx = rt.context_mut();
                println!("[kernel] Illegal instruction {:x} in {:#x}, core dumped.", a, ctx.sepc);
                rt.prepare_next_app(app::APP_MANAGER.prepare_next_app());
            },
            GeneratorState::Complete(()) => {
                sbi::shutdown()
            }
            // _ => todo!("handle more exceptions")
        }
    }
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

#[cfg(not(test))]
global_asm!(include_str!("link_app.S"));

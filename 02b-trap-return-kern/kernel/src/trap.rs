mod app_trap;
mod kernel_trap;

use riscv::register::stvec::{self, TrapMode};

pub use app_trap::{TrapContext, restore_trap};

pub fn set_app_trap() {
    let mut addr = app_trap::trap_entry as usize;
    if addr & 0x2 != 0 {
        addr += 0x2; // 必须对齐到4个字节
    }
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

// pub fn set_kernel_trap() {
//     let mut addr = kernel_trap::kernel_trap as usize;
//     if addr & 0x2 != 0 {
//         addr += 0x2; // 必须对齐到4个字节
//     }
//     unsafe { stvec::write(addr, TrapMode::Direct) };
// }

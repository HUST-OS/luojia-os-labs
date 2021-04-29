#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate trap_return_user;

#[no_mangle]
fn main() -> i32 {
    println!("Test user!");
    // pub fn write(fd: usize, buf: &[u8]) -> SyscallResult { sys_write(fd, buf) }
    let illegal_buffer = unsafe { core::slice::from_raw_parts(0x233333666666 as *const _, 10) };
    trap_return_user::write(1, illegal_buffer); // trigger EILL
    println!("After test user!");
    0
}

#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate batch_kernel_user;

#[no_mangle]
fn main() -> i32 {
    println!("Triggering illegal instruction!");
    unsafe { asm!("sret") }; // illegal
    0
}

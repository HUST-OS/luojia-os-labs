#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate batch_kernel_user;

#[no_mangle]
fn main() -> i32 {
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe { (0x0 as *mut u8).write_volatile(0); }
    0
}

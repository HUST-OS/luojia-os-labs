#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate yield_batch_kernel_user;

#[no_mangle]
fn main() -> i32 {
    println!("Hello, world!");
    0
}

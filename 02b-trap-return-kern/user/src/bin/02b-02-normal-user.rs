#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate trap_return_user;

#[no_mangle]
fn main() -> i32 {
    println!("Test user 2");
    0
}

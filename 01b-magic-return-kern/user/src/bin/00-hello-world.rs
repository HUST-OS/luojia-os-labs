#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate magic_return_user;

#[no_mangle]
fn main() -> i32 {
    println!("Hello, world!");
    0
}

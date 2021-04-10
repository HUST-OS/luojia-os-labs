#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate multi_program_user;

const WIDTH: usize = 10;
const HEIGHT: usize = 3;

#[no_mangle]
fn main() -> i32 {
    println!("Write C begin!");
    for i in 0..HEIGHT {
        for _ in 0..WIDTH { print!("C"); }
        println!(" [{}/{}]", i + 1, HEIGHT);
        multi_program_user::do_yield();
    }
    println!("Test write_c OK!");
    0
}

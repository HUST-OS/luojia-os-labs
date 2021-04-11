#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate complex_ctx_user;

struct FibonacciFuture {
    a: usize,
    b: usize,
    i: usize,
    cnt: usize,
}

impl FibonacciFuture {
    
    fn new(cnt: usize) -> FibonacciFuture {
        FibonacciFuture { a: 0, b: 1, i: 0, cnt }
    }
}
use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;


impl Future for FibonacciFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.i == self.cnt {
            println!("Fibonacci result: {}", self.a);
            Poll::Ready(())
        } else {
            let t = self.a;
            self.a += self.b;
            self.b = t;
            self.i += 1;
            println!("Fibonacci: i = {}, a = {}, b = {}", self.i, self.a, self.b);
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[no_mangle]
async fn main() -> i32 {
    println!("Now calculate fibonacci number.");
    FibonacciFuture::new(10).await;
    0
}

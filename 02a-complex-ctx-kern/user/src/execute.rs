use core::future::Future;
use alloc::sync::Arc;

use spin::Mutex;
use crate::task::{UserTask, RoundRobinScheduler};

pub fn execute_main(main: impl Future<Output = i32> + Send + Sync + 'static) -> i32 {
    let exit_code = Arc::new(Mutex::new(0));
    let exit_code_2 = exit_code.clone();
    let main_task = UserTask::new(async move {
        *exit_code_2.lock() = main.await;
    });
    let mut scheduler = RoundRobinScheduler::new();
    scheduler.push_task(Arc::new(main_task));
    // let mut stack = Vec::with_capacity(4 * 1024);
    loop {
        if let Some(task) = scheduler.pop_task() {
            task.mark_sleeping();
            let waker = woke::waker_ref(&task);
            let mut context = core::task::Context::from_waker(&*waker);
            let ret = task.future.lock().as_mut().poll(&mut context);
            if let core::task::Poll::Pending = ret {
                task.mark_ready();
                scheduler.push_task(task);
            } // else drop(task);
        } else {
            break
        }
    };
    let ans = *exit_code.lock();
    drop(exit_code);
    ans
}

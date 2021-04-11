use spin::Mutex;
use core::pin::Pin;
use alloc::boxed::Box;
use core::future::Future;
use alloc::sync::Arc;

pub struct UserTask {
    pub future: Mutex<Pin<Box<dyn Future<Output = ()> + 'static + Send + Sync>>>,
    sleeping: Mutex<bool>,
}

impl UserTask {
    pub fn new(f: impl Future<Output = ()> + 'static + Send + Sync) -> Self {
        Self {
            future: Mutex::new(Box::pin(f)),
            sleeping: Mutex::new(false),
        }
    }
    pub fn mark_ready(&self) {
        *self.sleeping.lock() = false;
    }
    pub fn mark_sleeping(&self) {
        *self.sleeping.lock() = true;
    }
    pub fn is_sleeping(&self) -> bool {
        *self.sleeping.lock()
    }
}

impl woke::Woke for UserTask {
    fn wake_by_ref(task: &Arc<Self>) {
        task.mark_ready();
    }
}

use alloc::collections::LinkedList;

pub struct RoundRobinScheduler {
    tasks: LinkedList<Arc<UserTask>>,
}

// todo: sleep, wake

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self { 
            tasks: LinkedList::new(),
        }
    }
    pub fn push_task(&mut self, task: Arc<UserTask>) {
        self.tasks.push_back(task)
    }
    pub fn pop_task(&mut self) -> Option<Arc<UserTask>> {
        while let Some(task) = self.tasks.pop_front() {
            if task.is_sleeping() {
                self.tasks.push_back(task);
            } else {
                return Some(task)
            }
        }
        None
    }
}

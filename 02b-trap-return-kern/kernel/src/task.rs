use core::cell::RefCell;
use crate::loader::{init_app_ctx, get_num_app};

const MAX_APP_NUM: usize = 16;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_cx_ptr: usize, // Option<*mut TaskContext>,
    pub task_status: TaskStatus,
}

impl TaskControlBlock {
    pub fn get_task_ctx_mut2(&mut self) -> *mut usize {
        &mut self.task_cx_ptr as *mut usize
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Uninitialized,
    Ready,
    Running,
    Finished,
}

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

unsafe impl Sync for TaskManager {}

impl TaskManager {
    pub fn run_first_task(&self) {
        self.inner.borrow_mut().tasks[0].task_status = TaskStatus::Running;
        let next_task_ctx = self.inner.borrow().tasks[0].task_cx_ptr;
        let _unused = 0;
        unsafe {
            switch_task(
                &_unused as *const _ as *mut _,
                next_task_ctx
            );
        }
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_finished(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Finished;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_ctx2 = inner.tasks[current].get_task_ctx_mut2();
            let next_task_ctx = inner.tasks[next].task_cx_ptr;
            core::mem::drop(inner);
            unsafe {
                switch_task(
                    current_task_ctx2,
                    next_task_ctx,
                );
            }
        } else {
            println!("All applications completed!");
            crate::sbi::shutdown()
        }
    }
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_finished();
    TASK_MANAGER.run_next_task();
}

lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [
            TaskControlBlock { task_cx_ptr: 0, task_status: TaskStatus::Uninitialized };
            MAX_APP_NUM
        ];
        for i in 0..num_app {
            tasks[i].task_cx_ptr = init_app_ctx(i) as *const _ as usize;
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }),
        }
    };
}

#[repr(C)]
#[derive(Debug)]
pub struct TaskContext {
    pub ra: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
}

impl TaskContext {
    pub fn goto_restore() -> Self {
        let mut ans: TaskContext = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        ans.ra = crate::trap::restore_trap as usize;
        ans
    }
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn switch_task(_cur_task_ctx2: *mut usize, _nxt_task_ctx: usize) {
    asm!(
        "addi   sp, sp, -13*8", 
        "sd     sp, 0(a0)", // *cur_task_ctx2 *(*mut usize) <- sp(usize)
        "sd     ra, 0*8(sp)
        sd      s0, 1*8(sp)
        sd      s1, 2*8(sp)
        sd      s2, 3*8(sp)
        sd      s3, 4*8(sp)
        sd      s4, 5*8(sp)
        sd      s5, 6*8(sp)
        sd      s6, 7*8(sp)
        sd      s7, 8*8(sp)
        sd      s8, 9*8(sp)
        sd      s9, 10*8(sp)
        sd      s10, 11*8(sp)
        sd      s11, 12*8(sp)",
        "mv     sp, a1", // sp <- nxt_task_ctx
        "ld     ra, 0*8(sp)
        ld      s0, 1*8(sp)
        ld      s1, 2*8(sp)
        ld      s2, 3*8(sp)
        ld      s3, 4*8(sp)
        ld      s4, 5*8(sp)
        ld      s5, 6*8(sp)
        ld      s6, 7*8(sp)
        ld      s7, 8*8(sp)
        ld      s8, 9*8(sp)
        ld      s9, 10*8(sp)
        ld      s10, 11*8(sp)
        ld      s11, 12*8(sp)",
        "addi   sp, sp, 13*8",
        "ret",
        options(noreturn)
    )
}

// #[naked]
// #[link_section = ".text"]
// unsafe extern "C" fn restore_task(_task_ctx: *const TaskContext) -> ! {
//     asm!(
//         "mv     sp, a0", 
//         "ld     ra, 0*8(sp)
//         ld      s0, 1*8(sp)
//         ld      s1, 2*8(sp)
//         ld      s2, 3*8(sp)
//         ld      s3, 4*8(sp)
//         ld      s4, 5*8(sp)
//         ld      s5, 6*8(sp)
//         ld      s6, 7*8(sp)
//         ld      s7, 8*8(sp)
//         ld      s8, 9*8(sp)
//         ld      s9, 10*8(sp)
//         ld      s10, 11*8(sp)
//         ld      s11, 12*8(sp)",
//         "addi   sp, sp, 13*8",
//         "ret",
//         options(noreturn)
//     )
// }

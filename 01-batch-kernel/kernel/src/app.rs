use core::cell::RefCell;
use crate::trap::{self, TrapContext};

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe { *cx_ptr = cx; }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: KernelStack = KernelStack { data: [0; KERNEL_STACK_SIZE] };
static USER_STACK: UserStack = UserStack { data: [0; USER_STACK_SIZE] };

pub struct AppManager {
    inner: RefCell<AppManagerInner>,
}

unsafe impl Sync for AppManager {}

impl AppManager {
    pub fn new() -> AppManager {
        AppManager {
            inner: RefCell::new({
                extern "C" { fn _num_app(); }
                let num_app_ptr = _num_app as usize as *const usize;
                let num_app = unsafe { num_app_ptr.read_volatile() };
                let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
                let app_start_raw: &[usize] = unsafe {
                    core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
                };
                app_start[..=num_app].copy_from_slice(app_start_raw);
                AppManagerInner {
                    num_app,
                    current_app: 0,
                    app_start,
                }
            }),
        }
    }

    pub fn print_app_info(&self) {
        let inner = self.inner.borrow();
        println!("[kernel] num_app = {}", inner.num_app);
        for i in 0..inner.num_app {
            println!("[kernel] app_{} [{:#x}, {:#x})", i, inner.app_start[i], inner.app_start[i + 1]);
        }
    } 

    pub fn run_next_app(&self) -> ! {
        {
            let inner = self.inner.borrow();
            let current_app = inner.get_current_app_index();
            unsafe {
                inner.load_app(current_app);
            }
        }
        self.inner.borrow_mut().move_to_next_app();
        unsafe {
            let ctx = KERNEL_STACK.push_context(
                trap::TrapContext::app_init_context(APP_BASE_ADDRESS, USER_STACK.get_sp())
            );
            trap::restore(ctx)
        }
    }
}

struct AppManagerInner {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManagerInner {
    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            panic!("All applications completed!");
        }
        println!("[kernel] Loading app_{}", app_id);
        // clear icache
        asm!("fence.i");
        // clear app area
        (APP_BASE_ADDRESS..APP_BASE_ADDRESS + APP_SIZE_LIMIT).for_each(|addr| {
            (addr as *mut u8).write_volatile(0);
        });
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id]
        );
        let app_dst = core::slice::from_raw_parts_mut(
            APP_BASE_ADDRESS as *mut u8,
            app_src.len()
        );
        app_dst.copy_from_slice(app_src);
    }

    pub fn get_current_app_index(&self) -> usize { 
        self.current_app 
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static::lazy_static! {
    pub static ref APP_MANAGER: AppManager = AppManager::new();
}

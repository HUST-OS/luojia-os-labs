#[cfg(not(test))]
global_asm!(include_str!("link_app.S"));

const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

fn get_base_addr(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub fn load_apps() {
    extern "C" { fn _num_app(); }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = unsafe { num_app_ptr.read_volatile() };
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };
    for i in 0..num_app {
        let base_addr = get_base_addr(i);
        let mem_range = base_addr .. base_addr + APP_SIZE_LIMIT;
        mem_range.for_each(|addr| unsafe {
            (addr as *mut u8).write_volatile(0)
        });
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe {
            core::slice::from_raw_parts_mut(base_addr as *mut u8, src.len())
        };
        println!("[kernel] app #{}: {:#x}..{:#x}", i, base_addr, base_addr + src.len());
        dst.copy_from_slice(src);
    }
    println!("[kernel] load app finished");
    unsafe { asm!("fence.i"); }
}

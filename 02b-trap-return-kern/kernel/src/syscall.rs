const MODULE_PROCESS: usize = 0x114514;
const FUNCTION_PROCESS_EXIT: usize = 0x1919810;
const FUNCTION_PROCESS_PANIC: usize = 0x11451419;

const MODULE_TEST_INTERFACE: usize = 0x233666;
const FUNCTION_TEST_WRITE: usize = 0x666233;

const MODULE_TASK: usize = 0x7777777;
const FUNCTION_TASK_YIELD: usize = 0x9999999;

pub enum SyscallOperation {
    Return(SyscallResult),
    Terminate(i32),
    UserPanic(Option<&'static str>, u32, u32, Option<&'static str>),
    Yield,
}

pub struct SyscallResult {
    pub code: usize,
    pub extra: usize,
}

pub fn syscall(module: usize, function: usize, args: [usize; 6], app_id: usize) -> SyscallOperation {
    println!("[KERNEL] SYSCALL {:x} {:x} {:x?}", module, function, args);
    match module {
        MODULE_PROCESS => do_process(function, args, app_id),
        MODULE_TEST_INTERFACE => do_test_interface(function, [args[0], args[1], args[2]]),
        MODULE_TASK => do_task(function),
        _ => panic!("Unknown syscall, module: {}, function: {}, args: {:?}", module, function, args),
    }
}

fn do_process(function: usize, args: [usize; 6], app_id: usize) -> SyscallOperation {
    match function {
        FUNCTION_PROCESS_EXIT => SyscallOperation::Terminate(args[0] as i32),
        FUNCTION_PROCESS_PANIC => { // [line as usize, col as usize, f_buf, f_len, m_buf, m_len]
            let [line, col, f_buf, f_len, m_buf, m_len] = args;
            let file_name = unsafe { user_buffer(app_id, f_buf, f_len) }
                .map(|s| core::str::from_utf8(s).unwrap());
            let msg = unsafe {  user_buffer(app_id, m_buf, m_len) }
                .map(|s| core::str::from_utf8(s).unwrap());
            SyscallOperation::UserPanic(file_name, line as u32, col as u32, msg)
        },
        _ => panic!("Unknown syscall PROCESS, function: {}, args: {:?}", function, args),
    }
}

fn do_test_interface(function: usize, args: [usize; 3]) -> SyscallOperation {
    match function {
        FUNCTION_TEST_WRITE => { // fd: usize, buffer: &[u8] fd, buffer.as_ptr() as usize, buffer.len()
            const STDOUT: usize = 1;
            let [fd, buf, len] = args;
            if fd == STDOUT {
                let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                SyscallOperation::Return(SyscallResult { code: 0, extra: len as usize })
            } else {
                panic!("Unsupported fd {}", fd);
            }
        },
        _ => panic!("Unknown syscall TEST_INTERFACE,function: {}, arg: {:?}", function, args),
    }
}

fn do_task(function: usize) -> SyscallOperation {
    match function {
        FUNCTION_TASK_YIELD => SyscallOperation::Yield,
        _ => panic!("Unknown syscall TASK, function: {}", function),
    }
}

unsafe fn user_buffer<'a>(app_id: usize, user_ptr: usize, len: usize) -> Option<&'a [u8]> {
    if user_ptr == 0 {
        None
    } else {
        let kernel_ptr = crate::loader::get_ptr(app_id, user_ptr);
        Some(core::slice::from_raw_parts(kernel_ptr as *const u8, len))
    }
}

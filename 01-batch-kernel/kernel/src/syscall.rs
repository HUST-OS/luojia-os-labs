const MODULE_PROCESS: usize = 0x114514;
const FUNCTION_PROCESS_EXIT: usize = 0x1919810;
const FUNCTION_PROCESS_PANIC: usize = 0x11451419;

const MODULE_TEST_INTERFACE: usize = 0x233666;
const FUNCTION_TEST_WRITE: usize = 0x666233;

pub enum SyscallOperation {
    Return(SyscallResult),
    Terminate(i32),
    UserPanic(Option<&'static str>, u32, u32, Option<&'static str>),
}

pub struct SyscallResult {
    pub code: usize,
    pub extra: usize,
}

pub fn syscall(module: usize, function: usize, args: [usize; 6]) -> SyscallOperation {
    match module {
        MODULE_PROCESS => do_process(function, args),
        MODULE_TEST_INTERFACE => do_test_interface(function, [args[0], args[1], args[2]]),
        _ => panic!("Unknown syscall, module: {}, function: {}, args: {:?}", module, function, args),
    }
}

fn do_process(function: usize, args: [usize; 6]) -> SyscallOperation {
    match function {
        FUNCTION_PROCESS_EXIT => SyscallOperation::Terminate(args[0] as i32),
        FUNCTION_PROCESS_PANIC => { // [line as usize, col as usize, f_buf, f_len, m_buf, m_len]
            let [line, col, f_buf, f_len, m_buf, m_len] = args;
            let file_name = if f_buf == 0 {
                None
            } else {
                let slice = unsafe { core::slice::from_raw_parts(f_buf as *const u8, f_len) };
                Some(core::str::from_utf8(slice).unwrap())
            };
            let msg = if m_buf == 0 {
                None
            } else {
                let slice = unsafe { core::slice::from_raw_parts(m_buf as *const u8, m_len) };
                Some(core::str::from_utf8(slice).unwrap())
            };
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

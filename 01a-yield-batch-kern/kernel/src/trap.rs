use riscv::register::{
    sstatus::{self, Sstatus, SPP},
    stvec::{self, TrapMode},
    scause::{self, Trap, Exception}, stval,
};
use crate::syscall::{syscall, SyscallOperation, fast_syscall};

pub fn init() {
    let mut addr = trap_entry as usize;
    if addr & 0x2 != 0 {
        addr += 0x2; // 必须对齐到4个字节
    }
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

#[repr(C)]
pub struct TrapContext {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
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
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        unsafe { sstatus::set_spp(SPP::User) };
        let mut ctx: TrapContext = unsafe { core::mem::MaybeUninit::uninit().assume_init() };
        ctx.sstatus = sstatus::read();
        ctx.sepc = entry;
        ctx.sp = sp;
        ctx
    }
}

extern "C" fn rust_trap_handler(ctx: &mut TrapContext) -> *mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            match syscall(ctx.a7, ctx.a6, [ctx.a0, ctx.a1, ctx.a2, ctx.a3, ctx.a4, ctx.a5]) {
                SyscallOperation::Return(ans) => {
                    ctx.a0 = ans.code;
                    ctx.a1 = ans.extra;
                    ctx.sepc = ctx.sepc.wrapping_add(4);
                }
                SyscallOperation::Terminate(code) => {
                    println!("[Kernel] Process returned with code {}", code);
                    crate::app::APP_MANAGER.run_next_app();
                }
                SyscallOperation::UserPanic(file, line, col, msg) => {
                    let file = file.unwrap_or("<no file>");
                    let msg = msg.unwrap_or("<no message>");
                    println!("[Kernel] User process panicked at '{}', {}:{}:{}", msg, file, line, col);
                    crate::app::APP_MANAGER.run_next_app();
                }
            }
        }
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, core dumped.");
            crate::app::APP_MANAGER.run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            crate::app::APP_MANAGER.run_next_app();
        }
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    ctx
}

#[repr(C)]
#[derive(Debug)]
pub struct FastContext {
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
}

extern "C" fn rust_fast_syscall(ctx: &FastContext) -> *mut TrapContext {
    fast_syscall(ctx.a6, [ctx.a0, ctx.a1, ctx.a2, ctx.a3, ctx.a4, ctx.a5]);
    crate::app::APP_MANAGER.run_next_app();
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn restore(_ctx: *mut TrapContext) -> ! {
    asm!(
        "mv     sp, a0",
        "ld      t0, 31*8(sp)
        ld      t1, 32*8(sp)
        ld      t2, 1*8(sp)
        csrw    sstatus, t0
        csrw    sepc, t1
        csrw    sscratch, t2",
        "ld     x1, 0*8(sp)
        ld      x3, 2*8(sp)
        ld      x4, 3*8(sp)
        ld      x5, 4*8(sp)
        ld      x6, 5*8(sp)
        ld      x7, 6*8(sp)
        ld      x8, 7*8(sp)
        ld      x9, 8*8(sp)
        ld      x10, 9*8(sp)
        ld      x11, 10*8(sp)
        ld      x12, 11*8(sp)
        ld      x13, 12*8(sp)
        ld      x14, 13*8(sp)
        ld      x15, 14*8(sp)
        ld      x16, 15*8(sp)
        ld      x17, 16*8(sp)
        ld      x18, 17*8(sp)
        ld      x19, 18*8(sp)
        ld      x20, 19*8(sp)
        ld      x21, 20*8(sp)
        ld      x22, 21*8(sp)
        ld      x23, 22*8(sp)
        ld      x24, 23*8(sp)
        ld      x25, 24*8(sp)
        ld      x26, 25*8(sp)
        ld      x27, 26*8(sp)
        ld      x28, 27*8(sp)
        ld      x29, 28*8(sp)
        ld      x30, 29*8(sp)
        ld      x31, 30*8(sp)",
        "addi   sp, sp, 33*8",
        "csrrw  sp, sscratch, sp",
        "sret",
        options(noreturn)
    )
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn trap_entry() -> ! {
    asm!(
        ".p2align 2",
        "csrrw  sp, sscratch, sp",
        "bnez   a7, 1f", // 不用保存上下文的条件：a7=0 且 scause=8
        "csrr   a7, scause", // 此时恰好a7的值已经使用过了
        "addi   a7, a7, -8
        bnez    a7, 1f",
        "addi   sp, sp, -7*8",
        "sd     a0, 0*8(sp)
        sd      a1, 1*8(sp)
        sd      a2, 2*8(sp)
        sd      a3, 3*8(sp)
        sd      a4, 4*8(sp)
        sd      a5, 5*8(sp)
        sd      a6, 6*8(sp)",
        "mv     a0, sp",
        "call   {fast_syscall}",
        "j      2f", // 跳转到恢复上下文
        "1:", // 需要保存上下文
        "addi   sp, sp, -33*8",
        "sd     x1, 0*8(sp)
        sd      x3, 2*8(sp)
        sd      x4, 3*8(sp)
        sd      x5, 4*8(sp)
        sd      x6, 5*8(sp)
        sd      x7, 6*8(sp)
        sd      x8, 7*8(sp)
        sd      x9, 8*8(sp)
        sd      x10, 9*8(sp)
        sd      x11, 10*8(sp)
        sd      x12, 11*8(sp)
        sd      x13, 12*8(sp)
        sd      x14, 13*8(sp)
        sd      x15, 14*8(sp)
        sd      x16, 15*8(sp)
        sd      x17, 16*8(sp)
        sd      x18, 17*8(sp)
        sd      x19, 18*8(sp)
        sd      x20, 19*8(sp)
        sd      x21, 20*8(sp)
        sd      x22, 21*8(sp)
        sd      x23, 22*8(sp)
        sd      x24, 23*8(sp)
        sd      x25, 24*8(sp)
        sd      x26, 25*8(sp)
        sd      x27, 26*8(sp)
        sd      x28, 27*8(sp)
        sd      x29, 28*8(sp)
        sd      x30, 29*8(sp)
        sd      x31, 30*8(sp)",
        "csrr   t0, sstatus
        sd      t0, 31*8(sp)",
        "csrr   t1, sepc
        sd      t1, 32*8(sp)",
        "csrr   t2, sscratch
        sd      t2, 1*8(sp)",
        "mv     a0, sp
        call    {trap_handler}",
        "2:", // 恢复上下文 
        "mv     sp, a0",
        "ld     t0, 31*8(sp)
        ld      t1, 32*8(sp)
        ld      t2, 1*8(sp)
        csrw    sstatus, t0
        csrw    sepc, t1
        csrw    sscratch, t2",
        "ld     x1, 0*8(sp)
        ld      x3, 2*8(sp)
        ld      x4, 3*8(sp)
        ld      x5, 4*8(sp)
        ld      x6, 5*8(sp)
        ld      x7, 6*8(sp)
        ld      x8, 7*8(sp)
        ld      x9, 8*8(sp)
        ld      x10, 9*8(sp)
        ld      x11, 10*8(sp)
        ld      x12, 11*8(sp)
        ld      x13, 12*8(sp)
        ld      x14, 13*8(sp)
        ld      x15, 14*8(sp)
        ld      x16, 15*8(sp)
        ld      x17, 16*8(sp)
        ld      x18, 17*8(sp)
        ld      x19, 18*8(sp)
        ld      x20, 19*8(sp)
        ld      x21, 20*8(sp)
        ld      x22, 21*8(sp)
        ld      x23, 22*8(sp)
        ld      x24, 23*8(sp)
        ld      x25, 24*8(sp)
        ld      x26, 25*8(sp)
        ld      x27, 26*8(sp)
        ld      x28, 27*8(sp)
        ld      x29, 28*8(sp)
        ld      x30, 29*8(sp)
        ld      x31, 30*8(sp)",
        "addi   sp, sp, 33*8",
        "csrrw  sp, sscratch, sp",
        "sret",
        trap_handler = sym rust_trap_handler,
        fast_syscall = sym rust_fast_syscall,
        options(noreturn)
    )
}

use riscv::register::{
    sstatus::{self, Sstatus, SPP},
    scause::{self, Trap, Exception},
    stvec::{self, TrapMode}, stval,
};
const USER_STACK_SIZE: usize = 4096 * 2;

pub fn init() {
    let mut addr = crate::executor::from_user_save as usize;
    if addr & 0x2 != 0 {
        addr += 0x2; // 必须对齐到4个字节
    }
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

#[repr(C)]
pub struct Runtime {
    context: UserContext, 
}

#[repr(align(4))] // 防止非对齐访问
struct UserStack([u8; USER_STACK_SIZE]);

static mut USER_STACK: UserStack = UserStack([0; USER_STACK_SIZE]);

impl Runtime {
    pub fn new_user() -> Self {
        let context: UserContext = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        let mut ans = Runtime { context };
        ans.reset();
        ans
    }

    pub fn reset(&mut self) {
        let stack_start = unsafe { (&mut USER_STACK as *mut _ as *mut u8).offset(USER_STACK_SIZE as isize) };
        self.context.sp = stack_start as usize;
        unsafe { sstatus::set_spp(SPP::User) };
        self.context.sstatus = sstatus::read();
        self.context.kernel_stack = 0x233333666666; // 将会被resume函数覆盖
    }

    pub fn context_mut(&mut self) -> &mut UserContext {
        &mut self.context
    }

    pub fn resume(&mut self) -> ResumeResult {
        unsafe { do_resume(&mut self.context as *mut _) }
    }
}

#[repr(C)]
pub enum ResumeResult<'a> {
    Syscall(&'a mut UserContext),
    LoadAccessFault(usize),
    StoreAccessFault(usize),
    IllegalInstruction(usize), // 这里暂时不能有两个参数，两个参数会把返回值存栈上，导致出现一些问题
}

#[derive(Debug)]
#[repr(C)]
pub struct UserContext {
    pub ra: usize,
    pub sp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_stack: usize, // 19
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn do_resume<'a>(_user_context: *mut UserContext) -> ResumeResult<'a> {
    asm!("j     {from_kernel_save}", from_kernel_save = sym from_kernel_save, options(noreturn))
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn from_kernel_save(_user_context: *mut UserContext) -> ! {
    asm!( // sp:内核栈顶
        "addi   sp, sp, -15*8", // sp:内核栈顶
        // 进入函数之前，已经保存了调用者寄存器，应当保存被调用者寄存器
        "sd     ra, 0*8(sp)
        sd      gp, 1*8(sp)
        sd      tp, 2*8(sp)
        sd      s0, 3*8(sp)
        sd      s1, 4*8(sp)
        sd      s2, 5*8(sp)
        sd      s3, 6*8(sp)
        sd      s4, 7*8(sp)
        sd      s5, 8*8(sp)
        sd      s6, 9*8(sp)
        sd      s7, 10*8(sp)
        sd      s8, 11*8(sp)
        sd      s9, 12*8(sp)
        sd      s10, 13*8(sp)
        sd      s11, 14*8(sp)", 
        // a0:用户上下文
        "j      {to_user_restore}",
        to_user_restore = sym to_user_restore,
        options(noreturn)
    )
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn to_user_restore(_user_context: *mut UserContext) -> ! {
    asm!( // a0:用户上下文
        "sd     sp, 19*8(a0)", // 内核栈顶放进用户上下文
        "csrw   sscratch, a0", // 新sscratch:用户上下文
        // sscratch:用户上下文
        "mv     sp, a0", // 新sp:用户上下文
        "ld     t0, 17*8(sp)
        ld      t1, 18*8(sp)
        csrw    sstatus, t0
        csrw    sepc, t1",
        "ld     ra, 0*8(sp)
        ld      t0, 2*8(sp)
        ld      t1, 3*8(sp)
        ld      t2, 4*8(sp)
        ld      t3, 5*8(sp)
        ld      t4, 6*8(sp)
        ld      t5, 7*8(sp)
        ld      t6, 8*8(sp)
        ld      a0, 9*8(sp)
        ld      a1, 10*8(sp)
        ld      a2, 11*8(sp)
        ld      a3, 12*8(sp)
        ld      a4, 13*8(sp)
        ld      a5, 14*8(sp)
        ld      a6, 15*8(sp)
        ld      a7, 16*8(sp)",
        "ld     sp, 1*8(sp)", // 新sp:用户栈
        // sp:用户栈, sscratch:用户上下文
        "sret",
        options(noreturn)
    )
}

// 中断开始

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn from_user_save() -> ! {
    asm!( // sp:用户栈,sscratch:用户上下文
        ".p2align 2",
        "csrrw  sp, sscratch, sp", // 新sscratch:用户栈, 新sp:用户上下文
        "sd     ra, 0*8(sp)
        sd      t0, 2*8(sp)
        sd      t1, 3*8(sp)
        sd      t2, 4*8(sp)
        sd      t3, 5*8(sp)
        sd      t4, 6*8(sp)
        sd      t5, 7*8(sp)
        sd      t6, 8*8(sp)
        sd      a0, 9*8(sp)
        sd      a1, 10*8(sp)
        sd      a2, 11*8(sp)
        sd      a3, 12*8(sp)
        sd      a4, 13*8(sp)
        sd      a5, 14*8(sp)
        sd      a6, 15*8(sp)
        sd      a7, 16*8(sp)",
        "csrr   t0, sstatus
        sd      t0, 17*8(sp)",
        "csrr   t1, sepc
        sd      t1, 18*8(sp)",
        // sscratch:用户栈,sp:用户上下文
        "csrrw  t2, sscratch, sp", // 新sscratch:用户上下文,t2:用户栈
        "sd     t2, 1*8(sp)", // 保存用户栈
        "mv     a0, sp", // a0:用户上下文
        "ld     sp, 19*8(sp)", // sp:内核栈
        "la     ra, {to_kernel_restore}",
        "j      {user_trap_handler}",
        user_trap_handler = sym user_trap_handler,
        to_kernel_restore = sym to_kernel_restore,
        options(noreturn)
    )
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn to_kernel_restore() -> ! {
    asm!( // sscratch:用户上下文
        "csrr   sp, sscratch", // sp:用户上下文
        "ld     sp, 19*8(sp)", // sp:内核栈
        "ld     ra, 0*8(sp)
        ld      gp, 1*8(sp)
        ld      tp, 2*8(sp)
        ld      s0, 3*8(sp)
        ld      s1, 4*8(sp)
        ld      s2, 5*8(sp)
        ld      s3, 6*8(sp)
        ld      s4, 7*8(sp)
        ld      s5, 8*8(sp)
        ld      s6, 9*8(sp)
        ld      s7, 10*8(sp)
        ld      s8, 11*8(sp)
        ld      s9, 12*8(sp)
        ld      s10, 13*8(sp)
        ld      s11, 14*8(sp)", 
        "addi   sp, sp, 15*8", // sp:内核栈顶
        "jr     ra", // 其实就是ret
        options(noreturn)
    )
}

extern "C" fn user_trap_handler(user_ctx: &mut UserContext) -> ResumeResult<'_> {
    let stval = stval::read();
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => ResumeResult::Syscall(user_ctx),
        Trap::Exception(Exception::LoadFault) => ResumeResult::LoadAccessFault(stval),
        Trap::Exception(Exception::StoreFault) => ResumeResult::StoreAccessFault(stval),
        Trap::Exception(Exception::IllegalInstruction) => ResumeResult::IllegalInstruction(stval),
        _ => panic!("todo: handle more exceptions!")
    }
}
// 返回值是一个复杂的结构体。存寄存器里好像没问题，关键是有些函数它返回值存栈上，就很离谱

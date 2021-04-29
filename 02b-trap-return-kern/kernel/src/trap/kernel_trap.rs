use riscv::register::{
    sstatus::Sstatus,
    scause::{self, Trap, Exception}, stval, sepc,
};

unsafe extern "C" fn kernel_trap_handler(ctx: &mut KernelTrapContext) {
    println!("{:x?}", ctx);
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::StoreFault) => {
            println!("[kernel] User provided illegal address {:#x}, kill this process", stval);
            sepc::write(crate::task::exit_current_and_run_next as usize);
        },
        _ => {
            panic!("Kernel trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn kernel_trap() -> ! {
    asm!(
        ".p2align 2",
        "addi   sp, sp, -17*8",
        "sd     ra, 0*8(sp)
        sd      t0, 1*8(sp)
        sd      t1, 2*8(sp)
        sd      t2, 3*8(sp)
        sd      t3, 4*8(sp)
        sd      t4, 5*8(sp)
        sd      t5, 6*8(sp)
        sd      t6, 7*8(sp)
        sd      a0, 8*8(sp)
        sd      a1, 9*8(sp)
        sd      a2, 10*8(sp)
        sd      a3, 11*8(sp)
        sd      a4, 12*8(sp)
        sd      a5, 13*8(sp)
        sd      a6, 14*8(sp)
        sd      a7, 15*8(sp)",
        "csrr   t0, sstatus
        sd      t0, 16*8(sp)",
        "mv     a0, sp
        la      ra, {trap_restore}
        j       {trap_handler}",
        trap_handler = sym kernel_trap_handler,
        trap_restore = sym kernel_restore,
        options(noreturn)
    )
}

#[repr(C)]
#[derive(Debug)]
pub struct KernelTrapContext {
    pub ra: usize,
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
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn kernel_restore() -> ! {
    asm!(
        "ld     t0, 16*8(sp)
        csrw    sstatus, t0",
        "ld     ra, 0*8(sp)
        ld      t0, 1*8(sp)
        ld      t1, 2*8(sp)
        ld      t2, 3*8(sp)
        ld      t3, 4*8(sp)
        ld      t4, 5*8(sp)
        ld      t5, 6*8(sp)
        ld      t6, 7*8(sp)
        ld      a0, 8*8(sp)
        ld      a1, 9*8(sp)
        ld      a2, 10*8(sp)
        ld      a3, 11*8(sp)
        ld      a4, 12*8(sp)
        ld      a5, 13*8(sp)
        ld      a6, 14*8(sp)
        ld      a7, 15*8(sp)",
        "addi   sp, sp, 17*8",
        "sret",
        options(noreturn)
    )
}

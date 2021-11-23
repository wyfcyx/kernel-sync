extern crate alloc;

use alloc::sync::Arc;
use lazy_static::*;

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Cpu {
    pub noff: i32,              // Depth of push_off() nesting.
    pub interrupt_enable: bool, // Were interrupts enabled before push_off()?
}

lazy_static! {
    pub static ref CPUS: [Arc<Cpu>; 2] = [Arc::new(Cpu::default()), Arc::new(Cpu::default())];
}

pub fn mycpu() -> Arc<Cpu> {
    return CPUS[0].clone();
}

// push_off/pop_off are like intr_off()/intr_on() except that they are matched:
// it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
// are initially off, then push_off, pop_off leaves them off.
pub(crate) fn push_off() {
    let old = intr_get();
    disable_intr();
    let mut cpu = mycpu();
    let cpu_ref = Arc::get_mut(&mut cpu).unwrap();
    if cpu_ref.noff == 0 {
        cpu_ref.interrupt_enable = old;
    }
    cpu_ref.noff += 1;
}

pub(crate) fn pop_off() {
    let mut cpu = mycpu();
    let cpu_ref = Arc::get_mut(&mut cpu).unwrap();
    if intr_get() {
        panic!("pop_off - interruptible");
    }

    if cpu_ref.noff < 1 {
        panic!("pop off");
    }

    cpu_ref.noff -= 1;
    if cpu_ref.noff == 0 && cpu_ref.interrupt_enable {
        enable_intr();
    }
}

pub fn enable_intr() {
    unsafe { riscv::register::sstatus::set_sie() };
}

pub fn disable_intr() {
    unsafe { riscv::register::sstatus::clear_sie() };
}

pub fn intr_get() -> bool {
    riscv::register::sstatus::read().sie()
}

pub(crate) fn cpuid() -> u8 {
    let mut tp: usize;
    unsafe {
        asm!("mv {0}, tp", out(reg) tp);
    };
    tp as u8
}

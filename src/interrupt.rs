extern crate alloc;

use core::cell::{RefCell, RefMut};

use lazy_static::*;

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Cpu {
    pub noff: i32,              // Depth of push_off() nesting.
    pub interrupt_enable: bool, // Were interrupts enabled before push_off()?
}
pub struct SafeRefCell<T>(RefCell<T>);

unsafe impl<Cpu> Sync for SafeRefCell<Cpu> {}

impl<T> SafeRefCell<T> {
    fn new(t: T) -> Self {
        Self(RefCell::new(t))
    }
}

lazy_static! {
    pub static ref CPUS: [SafeRefCell<Cpu>; 10] = [
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default()),
        SafeRefCell::new(Cpu::default())
    ]; // TODO: remove hard code logic.
}

/// return id of current cpu, it requires kernel maintaining cpuid in tp
/// register.
pub(crate) fn cpu_id() -> u8 {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))] {
            let mut cpu_id: i64;
            unsafe {
                asm!("mv {0}, tp", out(reg) cpu_id);
            }
            (cpu_id & 0xff) as u8
        }else {
            0
        }
    }
}

pub fn mycpu() -> RefMut<'static, Cpu> {
    return CPUS[cpu_id() as usize].0.borrow_mut();
}

// push_off/pop_off are like intr_off()/intr_on() except that they are matched:
// it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
// are initially off, then push_off, pop_off leaves them off.
pub(crate) fn push_off() {
    let old = intr_get();
    disable_intr();
    let mut cpu = mycpu();
    if cpu.noff == 0 {
        cpu.interrupt_enable = old;
    }
    cpu.noff += 1;
}

pub(crate) fn pop_off() {
    let mut cpu = mycpu();
    if intr_get() {
        panic!("pop_off - interruptible");
    }

    if cpu.noff < 1 {
        panic!("pop off");
    }

    cpu.noff -= 1;
    if cpu.noff == 0 && cpu.interrupt_enable {
        enable_intr();
    }
}

pub fn enable_intr() {
    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    unsafe {
        riscv::register::sstatus::set_sie()
    };
}

pub fn disable_intr() {
    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    unsafe {
        riscv::register::sstatus::clear_sie()
    };
}
pub fn intr_get() -> bool {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))] {
            riscv::register::sstatus::read().sie()
        }else {
            false
        }
    }
}

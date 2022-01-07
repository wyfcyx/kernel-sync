#![no_std]
#![feature(async_closure)]
#![feature(asm)]
#![feature(get_mut_unchecked)]

extern crate alloc;

pub mod interrupt;
pub mod mutex;
pub mod spinlock;

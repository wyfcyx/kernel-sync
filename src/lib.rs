#![no_std]

extern crate alloc;

mod interrupt;

pub mod mutex;
pub mod rwlock;
pub mod spin;

cfg_if::cfg_if! {
    if #[cfg(feature = "libos")] {
        pub use {mutex::*, rwlock::*};
    } else {
        pub use spin::*;
    }
}

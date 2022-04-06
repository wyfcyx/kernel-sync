#![no_std]

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        extern crate alloc;
        mod interrupt;
        pub mod mcslock;
        pub mod mutex;
        pub mod rwlock;
        pub use {mutex::*, rwlock::*, mcslock::*};
    } else {
        pub use spin::*;
    }
}

// #![no_std]
#![feature(async_closure)]
use log;

// extern "C" {
pub fn enable_intr() {
    log::info!("enable intr");
}
pub fn disable_intr() {
    log::info!("disenable intr");
}


// }

mod spinlock;
mod mutex;

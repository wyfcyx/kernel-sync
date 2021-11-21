mod spinlock;

extern "C" {
    pub(crate) fn enable_intr();
    pub(crate) fn disable_intr();
}

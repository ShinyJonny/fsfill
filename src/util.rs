/// Constructs a u64 value from its high and low u32 parts.
#[macro_export]
macro_rules! hilo {
    ($hi:expr, $lo:expr) => {
        ((($hi as u64) << 32) | $lo as u64)
    };
}

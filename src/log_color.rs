#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("\x1B[31m[Err] {}\x1B[0m", format_args!($($arg)*))
    };
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        eprintln!("\x1B[33m[Warn] {}\x1B[0m", format_args!($($arg)*))
    };
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        eprintln!("\x1B[32m[Info] {}\x1B[0m", format_args!($($arg)*))
    };
}
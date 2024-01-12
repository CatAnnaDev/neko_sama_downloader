#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("\x1B[31m[❌] {}\x1B[0m", format_args!($($arg)*))
    };
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        eprintln!("\x1B[33m[⚠️] {}\x1B[0m", format_args!($($arg)*))
    };
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        eprintln!("\x1B[32m[✅] {}\x1B[0m", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! dl_ready {
    ($($arg:tt)*) => {
        eprintln!("\x1B[94m[💾] {}\x1B[0m", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        eprintln!("\x1b[37m[🐛] {}\x1B[0m", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! header {
    ($($arg:tt)*) => {
        eprintln!("\x1b[35m{}\x1B[0m", format_args!($($arg)*))
    };
}

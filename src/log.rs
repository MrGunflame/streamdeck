#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        println!("[{}] [INFO] {}", ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), std::format_args!($($arg)+));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        println!("[DEBUG] {}", ::std::format_args!($($arg)+))
    };
}

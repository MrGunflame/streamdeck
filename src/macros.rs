#[macro_export]
macro_rules! load_icon {
    ($file:expr) => {{
        const BYTES: &[u8] = include_bytes!($file);

        ::image::load_from_memory(BYTES).unwrap()
    }};
}

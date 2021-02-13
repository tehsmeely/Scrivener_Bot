#![macro_use]
macro_rules! some_string {
    ($s:literal) => {
        Some(String::from($s))
    };
}

macro_rules! noop {
    ($($tt:tt)*) => {{
        // consume so as to not trigger clippy warnings
        let _ = format_args!($($tt)*);
    }};
}

#[macro_export]
macro_rules! log {
    () => {{}};
    ($($tt:tt)*) => {{ noop!($($tt)*) }};
}

#[macro_export]
macro_rules! indent {
    () => {{}};
    ($($tt:tt)*) => {{ noop!($($tt)*) }};
}

#[macro_export]
macro_rules! dedent {
    () => {{}};
    ($($tt:tt)*) => {{ noop!($($tt)*) }};
}

#[macro_export]
macro_rules! reset {
    () => {{}};
}

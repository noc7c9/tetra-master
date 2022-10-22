use std::sync::atomic::AtomicUsize;

#[allow(unused)]
pub const INDENT_WIDTH: usize = 3;

#[allow(unused)]
pub static INDENT: AtomicUsize = AtomicUsize::new(0);

#[macro_export]
macro_rules! log {
    () => {{
        println!();
    }};
    ($($tt:tt)*) => {{
        use std::sync::atomic::Ordering::SeqCst;
        use $crate::logging::{INDENT, INDENT_WIDTH};

        let indent = INDENT.load(SeqCst);
        println!("{:width$}{}", "", format_args!($($tt)*), width = indent * INDENT_WIDTH);
    }};
}

#[macro_export]
macro_rules! indent {
    () => {{
        use std::sync::atomic::Ordering::SeqCst;
        use $crate::logging::INDENT;

        INDENT.fetch_add(1, SeqCst);
    }};
    ($($tt:tt)*) => {{
        log!($($tt)*);
        indent!();
    }};
}

#[macro_export]
macro_rules! dedent {
    () => {{
        use std::sync::atomic::Ordering::SeqCst;
        use $crate::logging::INDENT;

        INDENT
            .fetch_update(SeqCst, SeqCst, |x| Some(x.saturating_sub(1)))
            .unwrap();
    }};
    ($($tt:tt)*) => {{
        dedent!();
        log!($($tt)*);
    }};
}

#[macro_export]
macro_rules! reset {
    () => {{
        use std::sync::atomic::Ordering::SeqCst;
        use $crate::logging::INDENT;

        INDENT.store(0, SeqCst);
    }};
}

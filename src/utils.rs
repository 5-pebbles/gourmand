macro_rules! no_std_debug_assert {
    ($condition:expr $(, $message:literal)? $(,)?) => {
        #[cfg(debug_assertions)]
        if !$condition {
            $crate::arch::write(
                2,
                concat!(
                    "assertion ", $("`", $message, "`",)? "failed: ",
                    stringify!($condition), "\n",
                    "  --> ",
                    file!(), ":",
                    line!(), ":",
                    column!(), "\n",
                ),
            );
            $crate::arch::exit(101);
        }
    };
}

pub(crate) use no_std_debug_assert;

// Styling
macro_rules! bold {
    ($($e:expr),+) => {
        concat!("\x1B[1m", $($e),+, "\x1B[22m")
    };
}

pub(crate) use bold;

macro_rules! underline {
    ($($e:expr),+) => {
        concat!("\x1B[4m", $($e),+, "\x1B[24m")
    };
}

pub(crate) use underline;

// Printing
macro_rules! syscall_print {
    ($($message:expr),+ $(,)?) => {
        $(
            $crate::arch::write(1, $message);
        )+
    };
}

pub(crate) use syscall_print;

macro_rules! syscall_debug_print {
    ($($message:expr),+ $(,)?) => {
        #[cfg(debug_assertions)]
        {
            $crate::linux::io_macros::syscall_print!($($message),+);
        }
    };
}

pub(crate) use syscall_debug_print;

macro_rules! syscall_println {
    ($($message:expr),+ $(,)?) => {
        $(
            $crate::arch::write(1, $message);
        )+
        $crate::arch::write(1, "\n");
    };
}

pub(crate) use syscall_println;

macro_rules! syscall_debug_println {
    ($($message:expr),+ $(,)?) => {
        #[cfg(debug_assertions)]
        {
            $crate::linux::io_macros::syscall_println!($($message),+);
        }
    };
}

pub(crate) use syscall_debug_println;

macro_rules! syscall_assert {
    ($condition:expr $(, $message:expr)? $(,)?) => {
        if !$condition {
            $crate::arch::write(2, "assertion ");

            $(
                $crate::arch::write(2, "`");
                $crate::arch::write(2, $message);
                $crate::arch::write(2, "`");
            )?

            $crate::arch::write(2, concat!(
                "failed: ",
                stringify!($condition), "\n",
                "  --> ",
                file!(), ":",
                line!(), ":",
                column!(), "\n",
            ));
            $crate::arch::exit(101);
        }
    };
}

pub(crate) use syscall_assert;

macro_rules! syscall_debug_assert {
    ($condition:expr $(, $message:expr)? $(,)?) => {
        #[cfg(debug_assertions)]
        {
            $crate::linux::io_macros::syscall_assert!($condition $(, $message)?);
        }
    };
}

pub(crate) use syscall_debug_assert;

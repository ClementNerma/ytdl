use lazy_static::lazy_static;

lazy_static! {
    pub static ref TERM_WIDTH: Option<u16> = termsize::get().map(|size| size.cols);
}

#[macro_export]
macro_rules! _format {
    ($color: ident => $message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        let msg = format!($message, $($params)*);

        let msg = match *$crate::logging::TERM_WIDTH {
            None => msg,
            Some(width) => msg.chars().take(width.into()).collect()
        };

        msg.$color()
    }}
}

#[macro_export]
macro_rules! fail {
    ($message: tt, $($params: tt)*) => {{
        eprintln!("{}", $crate::_format!(bright_red => $message, $($params)*));
        std::process::exit(1);
    }};

    ($message: tt) => {{
        fail!($message,)
    }};
}

#[macro_export]
macro_rules! error {
    ($message: tt, $($params: tt)*) => {{
        eprintln!("{}", $crate::_format!(bright_red => $message, $($params)*));
    }};

    ($message: tt) => {{
        error!($message,)
    }};
}

#[macro_export]
macro_rules! error_anyhow {
    ($error: expr) => {{
        use colored::Colorize;
        eprintln!("{}", format!("{:?}", $error).bright_red());
    }};
}

#[macro_export]
macro_rules! warn {
    ($message: tt, $($params: tt)*) => {{
        eprintln!("{}", $crate::_format!(bright_yellow => $message, $($params)*));
    }};

    ($message: tt) => {{
        warn!($message,)
    }};
}

#[macro_export]
macro_rules! info {
    ($message: tt, $($params: tt)*) => {{
        println!("{}", $crate::_format!(bright_blue => $message, $($params)*));
    }};

    ($message: tt) => {{
        info!($message,)
    }};
}

#[macro_export]
macro_rules! info_inline {
    ($message: tt, $($params: tt)*) => {{
        print!("{}", $crate::_format!(bright_blue => $message, $($params)*));
    }};

    ($message: tt) => {{
        info_inline!($message,)
    }};
}

#[macro_export]
macro_rules! success {
    ($message: tt, $($params: tt)*) => {{
        println!("{}", $crate::_format!(bright_green => $message, $($params)*));
    }};

    ($message: tt) => {{
        success!($message,)
    }};
}
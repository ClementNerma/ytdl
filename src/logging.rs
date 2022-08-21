use lazy_static::lazy_static;
use std::env::{var, VarError};

lazy_static! {
    pub static ref TERM_WIDTH: Option<usize> = match var("TERM_WIDTH") {
        Ok(width) => Some(
            width
                .parse::<usize>()
                .expect("Please provide a valid terminal width!")
        ),
        Err(VarError::NotUnicode(_)) => panic!("Please provide a valid terminal width!"),
        Err(VarError::NotPresent) => None,
    };
}

#[macro_export]
macro_rules! _format {
    ($color: ident => $message: tt, $($params: tt)*) => {{
        use colored::Colorize;
        let msg = format!($message, $($params)*);

        let msg = match *$crate::logging::TERM_WIDTH {
            None => msg,
            Some(width) => {
                // TODO: Remove this
                // HACK: to take into account the calculus error caused by colorization characters
                let width = (width as f64 * 1.2) as usize;
                msg.chars().take(width).collect()
            }
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

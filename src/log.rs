#![allow(unused)]

macro_rules! trace {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::trace!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04")))]
            let _ = ($( & $x ),*);
        }
    };
}
pub(super) use trace;

macro_rules! debug {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::debug!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04")))]
            let _ = ($( & $x ),*);
        }
    };
}
pub(super) use debug;

macro_rules! info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::info!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04")))]
            let _ = ($( & $x ),*);
        }
    };
}
pub(super) use info;

macro_rules! warn {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::warn!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04")))]
            let _ = ($( & $x ),*);
        }
    };
}
// pub(super) use warn;

macro_rules! error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log-04")]
            ::log::error!($s $(, $x)*);
            #[cfg(not(any(feature = "log-04")))]
            let _ = ($( & $x ),*);
        }
    };
}
pub(super) use error;

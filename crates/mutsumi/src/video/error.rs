use glib::prelude::*;
use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

macro_rules! mpv_error {
    ($( $name:ident = $code:expr ),* $(,)? ) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        pub enum MutsumiMpvError {
            $( $name ),*,
            Unknown(i32),
        }

        impl MutsumiMpvError {
            pub fn code(self) -> i32 {
                match self {
                    $( MutsumiMpvError::$name => $code, )*
                    MutsumiMpvError::Unknown(c) => c,
                }
            }

            pub fn from_code(code: i32) -> Self {
                match code {
                    $( $code => MutsumiMpvError::$name, )*
                    other => MutsumiMpvError::Unknown(other),
                }
            }
        }
    };
}

impl Display for MutsumiMpvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            MutsumiMpvError::Unknown(c) => write!(f, "Unknown({})", c),
            other => write!(f, "{:?} (code={})", other, other.code()),
        }
    }
}

impl Error for MutsumiMpvError {}

impl ToValue for MutsumiMpvError {
    fn to_value(&self) -> glib::Value {
        self.code().to_value()
    }

    fn value_type(&self) -> glib::Type {
        i32::static_type()
    }
}

unsafe impl<'a> glib::value::FromValue<'a> for MutsumiMpvError {
    type Checker = glib::value::GenericValueTypeChecker<i32>;

    unsafe fn from_value(value: &'a glib::Value) -> Self {
        let code = unsafe { i32::from_value(value) };
        MutsumiMpvError::from_code(code)
    }
}

mpv_error!(
    MpvSuccess = 0,
    MpvEventQueueFull = -1,
    MpvNoMem = -2,
    MpvUninitialized = -3,
    MpvInvalidParameter = -4,
    MpvOptionNotFound = -5,
    MpvOptionFormat = -6,
    MpvOptionError = -7,
    MpvPropertyNotFound = -8,
    MpvPropertyFormat = -9,
    MpvPropertyUnavailable = -10,
    MpvPropertyError = -11,
    MpvCommand = -12,
    MpvLoadingFailed = -13,
    MpvAoInitFailed = -14,
    MpvVoInitFailed = -15,
    MpvNothingToPlay = -16,
    MpvUnknownFormat = -17,
    MpvUnsupported = -18,
    MpvNotImplemented = -19,
    AreaNotInitialized = -101,
    ContextNotInitialized = -102,
);

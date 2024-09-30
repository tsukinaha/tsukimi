use std::{error, ffi::NulError, fmt, os::raw as ctype, rc::Rc, str::Utf8Error};

#[allow(missing_docs)]
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Error {
    Loadfile {
        error: Rc<Error>,
    },
    VersionMismatch {
        linked: ctype::c_ulong,
        loaded: ctype::c_ulong,
    },
    InvalidUtf8,
    Null,
    Raw(crate::MpvError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<NulError> for Error {
    fn from(_other: NulError) -> Error {
        Error::Null
    }
}

impl From<Utf8Error> for Error {
    fn from(_other: Utf8Error) -> Error {
        Error::InvalidUtf8
    }
}
impl From<crate::MpvError> for Error {
    fn from(other: crate::MpvError) -> Error {
        Error::Raw(other)
    }
}

impl error::Error for Error {}

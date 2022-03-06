// Copyright 2015-2020 Benjamin Fry <benjaminfry@me.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Error types for the crate

#![deny(missing_docs)]

use std::{fmt, io, sync};

use enum_as_inner::EnumAsInner;
use thiserror::Error;

#[cfg(feature = "backtrace")]
use crate::proto::{trace, ExtBacktrace};
use crate::{
    proto::error::{ProtoError, ProtoErrorKind},
    resolver::error::{ResolveError, ResolveErrorKind},
};

/// The error kind for errors that get returned in the crate
#[derive(Debug, EnumAsInner, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An error with an arbitrary message, referenced as &'static str
    #[error("{0}")]
    Message(&'static str),

    /// An error with an arbitrary message, stored as String
    #[error("{0}")]
    Msg(String),

    /// An error got returned from IO
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// An error got returned by the trust-dns-proto crate
    #[error("proto error: {0}")]
    Proto(#[from] ProtoError),

    /// An error got returned by the trust-dns-proto crate
    #[error("proto error: {0}")]
    Resolve(#[from] ResolveError),

    /// A request timed out
    #[error("request timed out")]
    Timeout,
}

/// The error type for errors that get returned in the crate
#[derive(Error, Clone, Debug)]
#[non_exhaustive]
pub struct Error {
    /// Kind of error that ocurred
    pub kind: Box<ErrorKind>,
    /// Backtrace to the source of the error
    #[cfg(feature = "backtrace")]
    pub backtrack: Option<ExtBacktrace>,
}

impl Error {
    /// Get the kind of the error
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        cfg_if::cfg_if! {
            if #[cfg(feature = "backtrace")] {
                if let Some(ref backtrace) = self.backtrack {
                    fmt::Display::fmt(&self.kind, f)?;
                    fmt::Debug::fmt(backtrace, f)
                } else {
                    fmt::Display::fmt(&self.kind, f)
                }
            } else {
                fmt::Display::fmt(&self.kind, f)
            }
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
            #[cfg(feature = "backtrace")]
            backtrack: trace!(),
        }
    }
}

impl From<&'static str> for Error {
    fn from(msg: &'static str) -> Self {
        ErrorKind::Message(msg).into()
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        ErrorKind::Msg(msg).into()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::TimedOut => ErrorKind::Timeout.into(),
            _ => ErrorKind::from(e).into(),
        }
    }
}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        match *e.kind() {
            ErrorKind::Timeout => Self::new(io::ErrorKind::TimedOut, e),
            _ => Self::new(io::ErrorKind::Other, e),
        }
    }
}

impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}

#[cfg(feature = "wasm-bindgen")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm-bindgen")))]
impl From<Error> for wasm_bindgen_crate::JsValue {
    fn from(e: Error) -> Self {
        js_sys::Error::new(&e.to_string()).into()
    }
}

impl Clone for ErrorKind {
    fn clone(&self) -> Self {
        use self::ErrorKind::*;
        match *self {
            Message(msg) => Message(msg),
            Msg(ref msg) => Msg(msg.clone()),
            Io(ref io) => Self::from(std::io::Error::from(io.kind())),
            Proto(ref proto) => Self::from(proto.clone()),
            Resolve(ref resolve) => Self::from(resolve.clone()),
            Timeout => Self::Timeout,
        }
    }
}

/// A trait marking a type which implements From<Error> and
/// std::error::Error types as well as Clone + Send
pub trait FromError: From<Error> + std::error::Error + Clone {}

impl<E> FromError for E where E: From<Error> + std::error::Error + Clone {}

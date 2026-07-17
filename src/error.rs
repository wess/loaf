//! Error type and result alias.

use std::fmt;

use crate::value::Value;

/// The result type used throughout loaf.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong talking to the runtime.
#[non_exhaustive]
pub enum Error {
    /// The runtime could not be created or an internal call failed.
    Runtime(String),

    /// Source code failed to parse or transpile.
    Syntax(String),

    /// JavaScript threw. `message` is the formatted error (message + stack when
    /// available); `value` is the thrown value itself when it could be kept.
    Js {
        message: String,
        value: Option<Value>,
    },

    /// A JS value could not be converted into the requested Rust type.
    FromJs {
        from: &'static str,
        to: &'static str,
        message: Option<String>,
    },

    /// A Rust value could not be converted into a JS value.
    IntoJs {
        from: &'static str,
        message: Option<String>,
    },

    /// A string crossing the boundary was not valid UTF-8.
    Utf8,

    /// An error raised by host (Rust) code from inside a callback, on its way
    /// back out to JavaScript.
    External(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Error {
    /// Wrap any standard error so it can be returned from a host callback and
    /// surface as a thrown JS exception.
    pub fn external<E>(err: E) -> Error
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error::External(Box::new(err))
    }

    /// Build a conversion error for `FromJs` implementations.
    pub fn from_js(from: &'static str, to: &'static str, message: impl Into<String>) -> Error {
        Error::FromJs {
            from,
            to,
            message: Some(message.into()),
        }
    }

    /// Build a conversion error for `IntoJs` implementations.
    pub fn into_js(from: &'static str, message: impl Into<String>) -> Error {
        Error::IntoJs {
            from,
            message: Some(message.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(m) => write!(f, "runtime error: {m}"),
            Error::Syntax(m) => write!(f, "syntax error: {m}"),
            Error::Js { message, .. } => write!(f, "uncaught JS exception: {message}"),
            Error::FromJs { from, to, message } => {
                write!(f, "cannot convert JS {from} to Rust {to}")?;
                if let Some(m) = message {
                    write!(f, ": {m}")?;
                }
                Ok(())
            }
            Error::IntoJs { from, message } => {
                write!(f, "cannot convert Rust {from} to a JS value")?;
                if let Some(m) = message {
                    write!(f, ": {m}")?;
                }
                Ok(())
            }
            Error::Utf8 => write!(f, "value was not valid UTF-8"),
            Error::External(e) => write!(f, "{e}"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to Display; the thrown Value doesn't add useful Debug.
        write!(f, "{self}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::External(e) => Some(&**e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(_: std::str::Utf8Error) -> Error {
        Error::Utf8
    }
}

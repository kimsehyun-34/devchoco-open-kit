//! The error type returned by [`download`](crate::download).

use std::fmt;
use std::io;

/// Why a download did not produce a file.
///
/// No variant carries the URL: download links are often bearer credentials, and the
/// underlying HTTP errors (which embed the URL) are reduced to a category on purpose.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The URL could not be parsed, has no host, or carries a user name or password.
    InvalidUrl,
    /// The scheme is not allowed (anything but `https`, or `http` when `https_only` is off).
    SchemeNotAllowed,
    /// The URL's host is not in the allow-list. Holds the offending host.
    HostNotAllowed(String),
    /// Nothing usable is left of the preferred file name after sanitising.
    InvalidName,
    /// The file name has a blocked extension (stored lower-case, without the dot).
    /// Nothing was requested; callers can fall back to another mechanism.
    Blocked(String),
    /// The destination name is taken and the policy is
    /// [`Collision::Fail`](crate::Collision::Fail).
    Exists,
    /// The server answered with a redirect that the redirect policy does not follow.
    Redirected(u16),
    /// The server answered with a status other than `200 OK`.
    Status(u16),
    /// The body is larger than `max_bytes`, declared or actual.
    TooLarge {
        /// The configured limit in bytes.
        limit: u64,
    },
    /// More bytes arrived than the server declared in `Content-Length`.
    SizeMismatch {
        /// Declared length.
        expected: u64,
        /// Bytes received when the transfer was stopped.
        received: u64,
    },
    /// The connection ended or broke before the whole body arrived.
    Truncated {
        /// Declared length, when the server sent one.
        expected: Option<u64>,
        /// Bytes received before the transfer ended.
        received: u64,
    },
    /// A connect or read timeout expired.
    Timeout,
    /// The request could not be sent or no response arrived (DNS, refused, TLS, reset).
    Network,
    /// The cancellation token fired.
    Cancelled,
    /// A local file operation failed (create, write, sync, length check or rename).
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidUrl => f.write_str("the URL is invalid or carries credentials"),
            Error::SchemeNotAllowed => f.write_str("the URL scheme is not allowed"),
            Error::HostNotAllowed(host) => write!(f, "host `{host}` is not in the allow-list"),
            Error::InvalidName => f.write_str("the file name is empty after sanitising"),
            Error::Blocked(ext) => write!(f, "files with the `.{ext}` extension are blocked"),
            Error::Exists => f.write_str("a file with this name already exists"),
            Error::Redirected(code) => write!(f, "refused a redirect (HTTP {code})"),
            Error::Status(code) => write!(f, "the server answered HTTP {code}"),
            Error::TooLarge { limit } => {
                write!(f, "the file is larger than the {limit} byte limit")
            }
            Error::SizeMismatch { expected, received } => {
                write!(f, "received {received} bytes but the server declared {expected}")
            }
            Error::Truncated { expected: Some(expected), received } => {
                write!(f, "the transfer ended after {received} of {expected} bytes")
            }
            Error::Truncated { expected: None, received } => {
                write!(f, "the transfer broke after {received} bytes")
            }
            Error::Timeout => f.write_str("the connection timed out"),
            Error::Network => f.write_str("the request failed"),
            Error::Cancelled => f.write_str("the download was cancelled"),
            Error::Io(error) => write!(f, "local file error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_never_contain_a_url() {
        let all = [
            Error::InvalidUrl,
            Error::SchemeNotAllowed,
            Error::HostNotAllowed("files.example".into()),
            Error::InvalidName,
            Error::Blocked("exe".into()),
            Error::Exists,
            Error::Redirected(302),
            Error::Status(404),
            Error::TooLarge { limit: 10 },
            Error::SizeMismatch { expected: 1, received: 2 },
            Error::Truncated { expected: Some(10), received: 3 },
            Error::Truncated { expected: None, received: 3 },
            Error::Timeout,
            Error::Network,
            Error::Cancelled,
            Error::Io(io::Error::other("disk full")),
        ];
        for error in all {
            let text = error.to_string();
            assert!(!text.is_empty());
            assert!(!text.contains("://"), "{text}");
        }
    }

    #[test]
    fn io_errors_convert_and_expose_their_source() {
        let error: Error = io::Error::other("boom").into();
        assert!(matches!(error, Error::Io(_)));
        assert!(std::error::Error::source(&error).is_some());
        assert!(std::error::Error::source(&Error::Cancelled).is_none());
    }
}

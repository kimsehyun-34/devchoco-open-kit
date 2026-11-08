//! Download a URL into a file without ever leaving a half-written or overwritten file behind.
//!
//! This first slice holds the file-name handling and the policy options. The download engine
//! that uses them follows in the next change.

// The helpers below are called by the engine, which is not part of this change yet.
#![allow(dead_code)]

mod names;
mod options;

pub use names::{sanitize_file_name, MAX_NAME_BYTES};
pub use options::{Collision, Options, RedirectPolicy, DEFAULT_BLOCKED_EXTENSIONS};

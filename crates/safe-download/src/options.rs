//! Download policy: which URLs are allowed, how big, how long, and what to do on a name clash.

use std::time::Duration;

/// Extensions of programs, scripts and shortcuts that [`Options::default`] refuses to save.
///
/// Lower-case, without the dot. Replace [`Options::blocked_extensions`] to change the list.
#[rustfmt::skip]
pub const DEFAULT_BLOCKED_EXTENSIONS: &[&str] = &[
    "appinstaller", "application", "appref-ms", "appx", "appxbundle", "bat", "chm", "cmd", "com",
    "command", "cpl", "diagcab", "dll", "dmg", "exe", "gadget", "hta", "img", "inf", "iso", "jar",
    "jnlp", "js", "jse", "library-ms", "lnk", "mpkg", "msc", "msh", "msh1", "msh2", "mshxml", "msi",
    "msix", "msixbundle", "msp", "pif", "pkg", "ps1", "ps1xml", "ps2", "ps2xml", "psc1", "psc2",
    "psm1", "reg", "scf", "scr", "settingcontent-ms", "sh", "sys", "url", "vb", "vbe", "vbs",
    "vhd", "vhdx", "website", "ws", "wsc", "wsf", "wsh", "xbap",
];

/// What to do with HTTP redirects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Never follow: any redirect is [`Error::Redirected`](crate::Error::Redirected).
    None,
    /// Follow at most this many hops. Every hop must pass the same host and scheme rules as
    /// the first URL; a hop that does not is reported as
    /// [`Error::Redirected`](crate::Error::Redirected).
    Limited(usize),
}

/// What to do when the destination file name is already taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collision {
    /// Keep the existing file and save as `name (1).ext`, `name (2).ext`, ...
    Number,
    /// Keep the existing file and fail with [`Error::Exists`](crate::Error::Exists).
    Fail,
}

/// Policy for one [`download`](crate::download) call.
///
/// The defaults are conservative: HTTPS only, no redirects, no hosts allowed (so at least
/// [`allowed_hosts`](Self::allowed_hosts) must be filled in), programs and scripts blocked.
#[derive(Clone, Debug)]
pub struct Options {
    /// Hosts the URL may point to, compared for exact equality (no wildcards, no subdomains).
    /// Use the ASCII/punycode form for internationalised names. IPv6 literals include brackets.
    /// An empty list refuses every URL.
    pub allowed_hosts: Vec<String>,
    /// Refuse `http://` URLs. Turn off only for local testing.
    pub https_only: bool,
    /// Largest body accepted, checked against `Content-Length` and against the bytes received.
    pub max_bytes: u64,
    /// Time allowed to establish a connection.
    pub connect_timeout: Duration,
    /// Longest silence tolerated between two reads. There is deliberately no overall limit:
    /// a large file on a slow line may take as long as it needs while bytes keep arriving.
    pub read_timeout: Duration,
    /// Redirect handling. See [`RedirectPolicy`].
    pub redirects: RedirectPolicy,
    /// Extensions (case-insensitive, with or without the dot) that are never saved.
    /// Checked on the sanitised name, before any request is made. Empty disables the check.
    pub blocked_extensions: Vec<String>,
    /// Behaviour when the file name is taken. An existing file is never overwritten.
    pub collision: Collision,
    /// Minimum time between two progress callbacks (the first and last are always sent).
    pub progress_interval: Duration,
    /// How many times the final rename is tried before giving up (a virus scanner or an
    /// indexer can hold a fresh file for a moment). At least one attempt is always made.
    pub rename_attempts: u32,
    /// Pause between rename attempts.
    pub rename_pause: Duration,
    /// On Windows, tag the file as downloaded from the internet (`Zone.Identifier`) so
    /// SmartScreen and Protected View still apply. Ignored on other platforms.
    pub mark_of_the_web: bool,
    /// Value of the `User-Agent` header.
    pub user_agent: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            https_only: true,
            max_bytes: 1 << 30,
            connect_timeout: Duration::from_secs(15),
            read_timeout: Duration::from_secs(60),
            redirects: RedirectPolicy::None,
            blocked_extensions: DEFAULT_BLOCKED_EXTENSIONS
                .iter()
                .map(|ext| (*ext).to_owned())
                .collect(),
            collision: Collision::Number,
            progress_interval: Duration::from_millis(250),
            rename_attempts: 5,
            rename_pause: Duration::from_millis(200),
            mark_of_the_web: true,
            user_agent: concat!("safe-download/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
}

impl Options {
    /// Default options that allow exactly the given hosts.
    pub fn for_hosts<I, S>(hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { allowed_hosts: hosts.into_iter().map(Into::into).collect(), ..Self::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_conservative() {
        let options = Options::default();
        assert!(options.allowed_hosts.is_empty());
        assert!(options.https_only);
        assert_eq!(options.redirects, RedirectPolicy::None);
        assert_eq!(options.collision, Collision::Number);
        assert!(options.blocked_extensions.iter().any(|ext| ext == "exe"));
        assert!(options.user_agent.starts_with("safe-download/"));
    }

    #[test]
    fn for_hosts_fills_only_the_allow_list() {
        let options = Options::for_hosts(["a.example", "b.example"]);
        assert_eq!(options.allowed_hosts, ["a.example", "b.example"]);
        assert!(options.https_only);
    }

    #[test]
    fn blocked_list_is_lower_case_and_dotless() {
        for ext in DEFAULT_BLOCKED_EXTENSIONS {
            assert_eq!(*ext, ext.to_ascii_lowercase());
            assert!(!ext.starts_with('.'));
        }
    }
}

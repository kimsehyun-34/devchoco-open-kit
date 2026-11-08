//! URL admission rules shared by the first request and every followed redirect.

use reqwest::Url;

use crate::error::Error;
use crate::options::Options;

/// The scheme and host checks, cloned into the redirect callback.
#[derive(Clone, Debug)]
pub(crate) struct UrlRules {
    hosts: Vec<String>,
    https_only: bool,
}

impl UrlRules {
    pub(crate) fn new(options: &Options) -> Self {
        let hosts =
            options.allowed_hosts.iter().map(|host| host.trim().to_ascii_lowercase()).collect();
        Self { hosts, https_only: options.https_only }
    }

    /// Parses `raw` and applies [`check`](Self::check).
    pub(crate) fn parse(&self, raw: &str) -> Result<Url, Error> {
        let url = Url::parse(raw).map_err(|_| Error::InvalidUrl)?;
        self.check(&url)?;
        Ok(url)
    }

    /// Scheme, credentials and host. The host is compared for exact equality, so
    /// `files.example.evil.test` and `evil.test@files.example` cannot pass for `files.example`.
    pub(crate) fn check(&self, url: &Url) -> Result<(), Error> {
        match url.scheme() {
            "https" => {}
            "http" if !self.https_only => {}
            _ => return Err(Error::SchemeNotAllowed),
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(Error::InvalidUrl);
        }
        let host = url.host_str().ok_or(Error::InvalidUrl)?;
        if self.hosts.iter().any(|allowed| allowed == host) {
            Ok(())
        } else {
            Err(Error::HostNotAllowed(host.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(https_only: bool) -> UrlRules {
        let mut options = Options::for_hosts(["files.example", " Cdn.Example "]);
        options.https_only = https_only;
        UrlRules::new(&options)
    }

    #[test]
    fn exact_hosts_over_https_pass() {
        let rules = rules(true);
        assert!(rules.parse("https://files.example/a/b.pdf").is_ok());
        assert!(rules.parse("https://FILES.example:8443/a?x=1#top").is_ok());
        // Entries are normalised, so stray case and spaces in the config do not matter.
        assert!(rules.parse("https://cdn.example/x").is_ok());
    }

    #[test]
    fn lookalike_and_nested_hosts_are_refused() {
        let rules = rules(true);
        for url in [
            "https://files.example.evil.test/a",
            "https://evil.files.example/a",
            "https://files.example.evil.test@files.example/a",
            "https://notfiles.example/a",
            "https://files.example./a",
            "https://other.example/a",
        ] {
            let result = rules.parse(url);
            assert!(
                matches!(result, Err(Error::HostNotAllowed(_)) | Err(Error::InvalidUrl)),
                "{url}: {result:?}"
            );
        }
    }

    #[test]
    fn credentials_are_refused() {
        let rules = rules(true);
        assert!(matches!(rules.parse("https://user:pw@files.example/a"), Err(Error::InvalidUrl)));
        assert!(matches!(rules.parse("https://user@files.example/a"), Err(Error::InvalidUrl)));
    }

    #[test]
    fn schemes_follow_the_https_only_switch() {
        let refused = rules(true).parse("http://files.example/a");
        assert!(matches!(refused, Err(Error::SchemeNotAllowed)));
        assert!(rules(false).parse("http://files.example/a").is_ok());
        for url in ["ftp://files.example/a", "file:///etc/passwd", "data:text/plain,hi"] {
            assert!(matches!(rules(false).parse(url), Err(Error::SchemeNotAllowed)), "{url}");
        }
    }

    #[test]
    fn garbage_and_empty_allow_lists_are_refused() {
        assert!(matches!(rules(true).parse("not a url"), Err(Error::InvalidUrl)));
        assert!(matches!(rules(true).parse(""), Err(Error::InvalidUrl)));
        let nobody = UrlRules::new(&Options::default());
        assert!(matches!(nobody.parse("https://files.example/a"), Err(Error::HostNotAllowed(_))));
    }
}

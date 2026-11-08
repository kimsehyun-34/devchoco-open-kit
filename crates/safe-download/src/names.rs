//! File names: sanitising, extension checks and exclusive creation.

use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::options::Collision;

/// Longest name produced, in UTF-8 bytes. Leaves room under the 255-byte/unit limits of common
/// file systems for the ` (n)` collision suffix and the `.part` temp suffix.
pub const MAX_NAME_BYTES: usize = 200;
/// Longest extension (dot included) that survives shortening.
const MAX_EXT_BYTES: usize = 16;
/// How many `name (n).ext` candidates are tried before giving up.
const MAX_CANDIDATES: u32 = 1000;

/// Characters Windows forbids in names; replaced by `_` on every platform.
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

/// Zero-width and bidirectional marks. They can make `report\u{202E}vsc.exe` display as
/// `report.exe.csv`, so they are dropped rather than replaced.
fn invisible(c: char) -> bool {
    const RANGES: &[(char, char)] = &[
        ('\u{061C}', '\u{061C}'),
        ('\u{200B}', '\u{200F}'),
        ('\u{2028}', '\u{202E}'),
        ('\u{2060}', '\u{2069}'),
        ('\u{FEFF}', '\u{FEFF}'),
    ];
    RANGES.iter().any(|&(low, high)| (low..=high).contains(&c))
}

/// Windows device names (`CON`, `COM1`, `LPT\u{b9}`, ...) are special with any extension.
fn is_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end().to_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$") {
        return true;
    }
    let chars: Vec<char> = stem.chars().collect();
    chars.len() == 4
        && (stem.starts_with("COM") || stem.starts_with("LPT"))
        && matches!(chars[3], '0'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}')
}

/// Splits at the last dot into `(stem, ".ext")`. A leading dot does not start an extension.
pub(crate) fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    }
}

fn trim_edges(text: &str) -> &str {
    text.trim_matches(|c: char| c == '.' || c.is_whitespace())
}

fn cut_at_boundary(text: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Caps the name at [`MAX_NAME_BYTES`], keeping a short extension and whole characters.
fn shorten(name: String) -> String {
    if name.len() <= MAX_NAME_BYTES {
        return name;
    }
    let (stem, extension) = split_extension(&name);
    let shortened = if extension.len() > 1 && extension.len() <= MAX_EXT_BYTES {
        let stem = trim_edges(cut_at_boundary(stem, MAX_NAME_BYTES - extension.len()));
        format!("{stem}{extension}")
    } else {
        cut_at_boundary(&name, MAX_NAME_BYTES).to_owned()
    };
    // Cutting can leave a trailing dot or space, which Windows would drop silently.
    shortened.trim_end_matches(|c: char| c == '.' || c.is_whitespace()).to_owned()
}

/// Turns an untrusted name into one that is safe to create on Windows, macOS and Linux.
///
/// * keeps only the last path component (`/` and `\` both separate), so `..\..\x` becomes `x`
/// * drops control, zero-width and bidirectional characters; replaces `< > : " | ? *` with `_`
///   (this also defuses `file.txt:Zone.Identifier` alternate-stream names)
/// * strips leading and trailing dots and whitespace
/// * prefixes `_` to Windows device names such as `con.txt` or `LPT1`
/// * caps the length at [`MAX_NAME_BYTES`] UTF-8 bytes, keeping a short extension
///
/// Returns `None` when nothing usable is left.
pub fn sanitize_file_name(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let replaced: String = last
        .chars()
        .filter(|c| !invisible(*c))
        .map(|c| if c.is_control() || FORBIDDEN.contains(&c) { '_' } else { c })
        .collect();
    let cleaned = trim_edges(&replaced);
    if cleaned.is_empty() {
        return None;
    }
    let mut cleaned = cleaned.to_owned();
    if is_device_name(&cleaned) {
        cleaned.insert(0, '_');
    }
    Some(shorten(cleaned))
}

/// The lower-case extension of `name` if it is in `blocked`.
///
/// Only the last extension counts, so `report.csv.exe` is blocked and a bare `exe` is not.
pub(crate) fn blocked_extension(name: &str, blocked: &[String]) -> Option<String> {
    let extension = split_extension(name).1.trim_start_matches('.');
    if extension.is_empty() {
        return None;
    }
    blocked
        .iter()
        .any(|entry| entry.trim().trim_start_matches('.').eq_ignore_ascii_case(extension))
        .then(|| extension.to_ascii_lowercase())
}

/// The paths to try for `name` in `dir`, in order: `name`, then `stem (1).ext`, `stem (2).ext`, ...
///
/// With [`Collision::Fail`] only `name` itself is offered.
pub(crate) fn candidates<'a>(
    dir: &'a Path,
    name: &'a str,
    collision: Collision,
) -> impl Iterator<Item = PathBuf> + 'a {
    let (stem, extension) = split_extension(name);
    let count = if collision == Collision::Fail { 1 } else { MAX_CANDIDATES };
    (0..count).map(move |n| {
        if n == 0 {
            dir.join(name)
        } else {
            dir.join(format!("{stem} ({n}){extension}"))
        }
    })
}

/// Creates `name` in `dir`, or `stem (n).ext` when it exists, and never replaces a file.
///
/// With [`Collision::Fail`] only `name` itself is tried and a taken name is an
/// [`ErrorKind::AlreadyExists`] error.
pub(crate) fn create_exclusive(
    dir: &Path,
    name: &str,
    collision: Collision,
) -> io::Result<(PathBuf, File)> {
    for candidate in candidates(dir, name, collision) {
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(ErrorKind::AlreadyExists, "too many files with this name"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let name = format!("safe-download-names-{label}-{}", std::process::id());
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sanitising_table() {
        let long_cjk = "\u{65e5}".repeat(300) + ".csv";
        let rows: Vec<(String, Option<String>)> = vec![
            // Plain names pass through, including spaces and non-ASCII.
            ("report 2026.csv".into(), Some("report 2026.csv".into())),
            ("caf\u{e9} menu.pdf".into(), Some("caf\u{e9} menu.pdf".into())),
            ("\u{65e5}\u{672c}\u{8a9e}.txt".into(), Some("\u{65e5}\u{672c}\u{8a9e}.txt".into())),
            // Folders are dropped, whichever separator was used.
            ("..\\..\\Windows\\evil.csv".into(), Some("evil.csv".into())),
            ("/etc/passwd".into(), Some("passwd".into())),
            ("a/b/c.txt".into(), Some("c.txt".into())),
            ("../".into(), None),
            ("dir/".into(), None),
            // Forbidden characters and alternate data streams.
            ("a<b>c:d\"e|f?g*h.txt".into(), Some("a_b_c_d_e_f_g_h.txt".into())),
            ("a.csv:Zone.Identifier".into(), Some("a.csv_Zone.Identifier".into())),
            ("tab\there.txt".into(), Some("tab_here.txt".into())),
            ("nul\0byte.txt".into(), Some("nul_byte.txt".into())),
            // Dots and spaces at the edges are removed (Windows would drop them silently).
            (".hidden".into(), Some("hidden".into())),
            ("..hidden.".into(), Some("hidden".into())),
            ("name. . ".into(), Some("name".into())),
            ("  padded.txt  ".into(), Some("padded.txt".into())),
            ("...".into(), None),
            ("   ".into(), None),
            ("".into(), None),
            // Invisible characters cannot disguise an extension.
            ("report\u{202E}vsc.exe".into(), Some("reportvsc.exe".into())),
            ("a\u{200B}b.txt".into(), Some("ab.txt".into())),
            ("\u{200B}".into(), None),
            // Windows device names, with or without extension, in any case.
            ("con".into(), Some("_con".into())),
            ("CON.txt".into(), Some("_CON.txt".into())),
            ("nul .csv".into(), Some("_nul .csv".into())),
            ("Aux.tar.gz".into(), Some("_Aux.tar.gz".into())),
            ("LPT1".into(), Some("_LPT1".into())),
            ("com\u{b9}.txt".into(), Some("_com\u{b9}.txt".into())),
            ("CONIN$".into(), Some("_CONIN$".into())),
            // Look-alikes that are not devices stay untouched.
            ("console.csv".into(), Some("console.csv".into())),
            ("COM10.csv".into(), Some("COM10.csv".into())),
            ("com.txt".into(), Some("com.txt".into())),
            ("nullable".into(), Some("nullable".into())),
            // Length cap in bytes, whole characters, extension kept.
            (long_cjk, Some("\u{65e5}".repeat(65) + ".csv")),
        ];
        for (input, expected) in rows {
            assert_eq!(sanitize_file_name(&input), expected, "input {input:?}");
        }
    }

    #[test]
    fn long_names_stay_within_the_byte_cap() {
        for name in [
            "a".repeat(500),
            "a".repeat(500) + ".pdf",
            "\u{1f600}".repeat(120),
            "x.".to_string() + &"b".repeat(400),
            "a".repeat(199) + ". " + &"b".repeat(20),
            "a".repeat(190) + ".tar.gz",
        ] {
            let short = sanitize_file_name(&name).unwrap();
            assert!(short.len() <= MAX_NAME_BYTES, "{} bytes for {name:.20}", short.len());
            assert!(!short.ends_with('.') && !short.ends_with(' '), "{short:?}");
            assert!(!short.is_empty());
        }
        let kept = sanitize_file_name(&("a".repeat(500) + ".pdf")).unwrap();
        assert!(kept.ends_with(".pdf"));
        assert_eq!(kept.len(), MAX_NAME_BYTES);
    }

    #[test]
    fn sanitising_is_idempotent() {
        for name in ["../a b.txt", "CON", "x:y.exe", "\u{202E}.hidden.", &"z".repeat(400)] {
            let once = sanitize_file_name(name).unwrap();
            assert_eq!(sanitize_file_name(&once).as_deref(), Some(once.as_str()), "{name:?}");
        }
    }

    #[test]
    fn only_the_last_extension_is_blocked() {
        let blocked: Vec<String> = ["exe", ".PS1", " lnk "].iter().map(|s| s.to_string()).collect();
        assert_eq!(blocked_extension("setup.exe", &blocked).as_deref(), Some("exe"));
        assert_eq!(blocked_extension("Setup.EXE", &blocked).as_deref(), Some("exe"));
        assert_eq!(blocked_extension("x.ps1", &blocked).as_deref(), Some("ps1"));
        assert_eq!(blocked_extension("a.lnk", &blocked).as_deref(), Some("lnk"));
        assert_eq!(blocked_extension("report.csv.exe", &blocked).as_deref(), Some("exe"));
        assert_eq!(blocked_extension("exe", &blocked), None);
        assert_eq!(blocked_extension("exe.txt", &blocked), None);
        assert_eq!(blocked_extension("notes", &blocked), None);
        assert_eq!(blocked_extension("setup.exe", &[]), None);
    }

    #[test]
    fn split_extension_ignores_leading_dots() {
        assert_eq!(split_extension("a.tar.gz"), ("a.tar", ".gz"));
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension("README"), ("README", ""));
    }

    #[test]
    fn candidates_count_up_and_the_fail_policy_offers_only_the_name() {
        let dir = Path::new("d");
        let numbered: Vec<PathBuf> =
            candidates(dir, "a.tar.gz", Collision::Number).take(3).collect();
        assert_eq!(
            numbered,
            [dir.join("a.tar.gz"), dir.join("a.tar (1).gz"), dir.join("a.tar (2).gz")]
        );
        let bare: Vec<PathBuf> = candidates(dir, "README", Collision::Number).take(2).collect();
        assert_eq!(bare, [dir.join("README"), dir.join("README (1)")]);
        assert_eq!(candidates(dir, "a.txt", Collision::Number).count(), MAX_CANDIDATES as usize);
        let only: Vec<PathBuf> = candidates(dir, "a.txt", Collision::Fail).collect();
        assert_eq!(only, [dir.join("a.txt")]);
    }

    #[test]
    fn exclusive_creation_numbers_and_never_replaces() {
        let dir = scratch("exclusive");
        let (first, _) = create_exclusive(&dir, "a.txt", Collision::Number).unwrap();
        std::fs::write(&first, b"one").unwrap();
        let (second, _) = create_exclusive(&dir, "a.txt", Collision::Number).unwrap();
        let (third, _) = create_exclusive(&dir, "a.txt", Collision::Number).unwrap();
        let (bare, _) = create_exclusive(&dir, "README", Collision::Number).unwrap();
        let (bare_again, _) = create_exclusive(&dir, "README", Collision::Number).unwrap();
        assert_eq!(second.file_name().unwrap(), "a (1).txt");
        assert_eq!(third.file_name().unwrap(), "a (2).txt");
        assert_eq!(bare.file_name().unwrap(), "README");
        assert_eq!(bare_again.file_name().unwrap(), "README (1)");
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fail_policy_reports_a_taken_name() {
        let dir = scratch("fail");
        std::fs::write(dir.join("a.txt"), b"keep").unwrap();
        let error = create_exclusive(&dir, "a.txt", Collision::Fail).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"keep");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

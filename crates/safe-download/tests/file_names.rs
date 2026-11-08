//! File-name edge cases: what an untrusted name turns into, and that the blocked-extension
//! check sees the sanitised name, so a disguised extension cannot slip past it.

mod common;

use common::{local_options, respond, run, Scratch, Server};
use safe_download::{download, sanitize_file_name, Error, Hooks, MAX_NAME_BYTES};

fn check(rows: &[(&str, Option<&str>)]) {
    for (input, expected) in rows {
        assert_eq!(sanitize_file_name(input).as_deref(), *expected, "input {input:?}");
    }
}

#[test]
fn drive_relative_and_network_paths_keep_only_the_file_name() {
    check(&[
        // "C:evil.txt" means "evil.txt in the current directory of drive C" to Windows.
        ("C:evil.txt", Some("C_evil.txt")),
        ("C:\\Windows\\System32\\drivers\\etc\\hosts", Some("hosts")),
        ("\\\\server\\share\\report.txt", Some("report.txt")),
        ("a/..\\b.txt", Some("b.txt")),
        ("./.hidden", Some("hidden")),
        ("dir\\", None),
        ("x/y\\z/", None),
        // The default data stream marker cannot survive: both colons are replaced.
        ("file.txt::$DATA", Some("file.txt__$DATA")),
    ]);
}

#[test]
fn every_kind_of_control_character_is_replaced() {
    check(&[
        ("a\u{7f}b.txt", Some("a_b.txt")),
        ("a\u{85}b.txt", Some("a_b.txt")),
        ("a\u{9b}b.txt", Some("a_b.txt")),
        ("line\r\nbreak.txt", Some("line__break.txt")),
        ("bell\u{7}.txt", Some("bell_.txt")),
    ]);
}

#[test]
fn windows_device_names_are_prefixed_but_look_alikes_are_not() {
    check(&[
        ("PRN", Some("_PRN")),
        ("prn.", Some("_prn")),
        ("COM9", Some("_COM9")),
        ("lpt0.txt", Some("_lpt0.txt")),
        ("COM\u{b2}", Some("_COM\u{b2}")),
        ("LPT\u{b3}.log", Some("_LPT\u{b3}.log")),
        ("conout$", Some("_conout$")),
        ("CLOCK$.txt", Some("_CLOCK$.txt")),
        ("  CON  ", Some("_CON")),
        ("COM", Some("COM")),
        ("LPT", Some("LPT")),
        ("COM1x", Some("COM1x")),
        ("AUX2", Some("AUX2")),
        ("my.con", Some("my.con")),
        ("CONSOLE", Some("CONSOLE")),
    ]);
}

#[test]
fn accents_wide_letters_and_emoji_are_kept_but_joiners_are_dropped() {
    check(&[
        ("e\u{301}.txt", Some("e\u{301}.txt")),
        ("\u{ff46}\u{ff55}\u{ff4c}\u{ff4c}.txt", Some("\u{ff46}\u{ff55}\u{ff4c}\u{ff4c}.txt")),
        ("\u{1f600}.png", Some("\u{1f600}.png")),
        // A zero-width joiner is one of the dropped invisible characters.
        ("\u{1f468}\u{200d}\u{1f469}.png", Some("\u{1f468}\u{1f469}.png")),
    ]);
}

#[test]
fn the_length_cap_is_exact_and_never_splits_a_character() {
    let exact = "a".repeat(MAX_NAME_BYTES - 4) + ".csv";
    assert_eq!(exact.len(), MAX_NAME_BYTES);
    assert_eq!(sanitize_file_name(&exact).as_deref(), Some(exact.as_str()));

    // One byte over: the stem gives up a byte, the extension stays.
    let over = "a".repeat(MAX_NAME_BYTES - 3) + ".csv";
    assert_eq!(sanitize_file_name(&over).as_deref(), Some(exact.as_str()));

    // An "extension" too long to keep is cut like the rest of the name.
    let long_ext = "a".repeat(190) + "." + &"b".repeat(30);
    let cut = sanitize_file_name(&long_ext).unwrap();
    assert_eq!(cut, "a".repeat(190) + "." + &"b".repeat(9));

    // Two-byte and four-byte characters: the cut moves back to a character boundary.
    let two_byte = "\u{e9}".repeat(101);
    assert_eq!(sanitize_file_name(&two_byte).unwrap(), "\u{e9}".repeat(100));
    let four_byte = "\u{1f600}".repeat(51);
    assert_eq!(sanitize_file_name(&four_byte).unwrap(), "\u{1f600}".repeat(50));

    // A cut that lands after a space must not leave the space on the end.
    let spaced = "a".repeat(195) + " " + &"b".repeat(10) + ".csv";
    assert_eq!(sanitize_file_name(&spaced).unwrap(), "a".repeat(195) + ".csv");
}

fn serve_ok() -> Server {
    Server::start(|_, stream| respond(stream, "200 OK", &[], b"data"))
}

#[test]
fn disguised_executable_names_are_blocked_before_any_request() {
    let server = serve_ok();
    let dir = Scratch::new("disguised");
    let options = local_options();
    let names = [
        "setup.exe .",
        "setup.exe...",
        "SETUP.EXE",
        "run.bat\u{200b}",
        "x.exe\u{202e}",
        "a.b\u{200d}.sh",
        "evil.exe\u{a0}",
        "evil.exe\u{3000}",
        "C:\\Users\\Public\\tool.cmd",
    ];
    for name in names {
        let result = run(download(&server.url("/file"), dir.path(), name, &options, Hooks::new()));
        assert!(matches!(result, Err(Error::Blocked(_))), "{name:?}: {result:?}");
    }
    assert_eq!(server.hits(), 0);
    assert!(dir.names().is_empty());
}

#[test]
fn names_neutralised_by_sanitising_are_saved_under_the_safe_name() {
    // The replaced character ends the extension, so these no longer name a program or script.
    let server = serve_ok();
    let dir = Scratch::new("inert");
    let options = local_options();
    let cases = [
        ("x.exe\0", "x.exe_"),
        ("run.bat\t", "run.bat_"),
        ("a.sh:Zone.Identifier", "a.sh_Zone.Identifier"),
    ];
    for (name, saved_as) in cases {
        let saved = run(download(&server.url("/file"), dir.path(), name, &options, Hooks::new()))
            .unwrap_or_else(|error| panic!("{name:?}: {error:?}"));
        assert_eq!(saved.path, dir.path().join(saved_as), "{name:?}");
    }
    assert_eq!(server.hits(), 3);
}

#[test]
fn a_name_with_nothing_usable_is_rejected_before_any_request() {
    let server = serve_ok();
    let dir = Scratch::new("empty-name");
    let options = local_options();
    for name in ["", "   ", "...", "../", "\u{200b}\u{202e}"] {
        let result = run(download(&server.url("/file"), dir.path(), name, &options, Hooks::new()));
        assert!(matches!(result, Err(Error::InvalidName)), "{name:?}: {result:?}");
    }
    assert_eq!(server.hits(), 0);
    assert!(dir.names().is_empty());
}

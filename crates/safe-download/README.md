# safe-download

Stream a URL into a file without ever leaving a half-written, truncated or overwritten
file behind. An async Rust library on top of `reqwest` (rustls) and `tokio`.

Whatever goes wrong - a URL that is not allowed, a blocked extension, a cancel, a dropped
connection, a full disk - the destination directory ends up exactly as it was. A file only
appears under its final name after every byte has arrived, reached the disk and been
measured there. The few exceptions (a killed process, a future dropped during the final
move, a file system without hard links) are listed under [Notes and limits](#notes-and-limits).

## Usage

```rust
use std::path::Path;
use safe_download::{download, CancelToken, Error, Hooks, Options};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Only this host may be contacted, over HTTPS, and redirects are not followed.
    let options = Options::for_hosts(["downloads.example.org"]);

    let cancel = CancelToken::new(); // give a clone to a "Cancel" button
    let hooks = Hooks::new()
        .on_progress(|bytes, total| println!("{bytes} of {total:?} bytes"))
        .with_cancel(cancel.clone());

    match download(
        "https://downloads.example.org/files/report.pdf",
        Path::new("."),
        "report.pdf",
        &options,
        hooks,
    )
    .await
    {
        Ok(saved) => println!("saved {} ({} bytes)", saved.path.display(), saved.bytes),
        // A program or script: nothing was requested, so the caller can fall back
        // (hand it to a browser, ask the user, reject it).
        Err(Error::Blocked(extension)) => println!("blocked .{extension}"),
        Err(other) => return Err(other),
    }
    Ok(())
}
```

(The example needs `tokio` with the `macros` and `rt-multi-thread` features in your own
crate; the library itself only needs a running Tokio runtime.)

## Safety properties

| Property | How it is enforced | Where it is tested |
| --- | --- | --- |
| Only listed hosts | Exact host equality against `allowed_hosts`; no wildcards, no subdomains; an empty list refuses everything; URLs with a user name or password are refused | `rules` unit tests, `hosts_outside_the_allow_list_are_refused_before_any_request` |
| HTTPS only | `https_only` (default on) rejects `http://` and every other scheme before connecting | `plain_http_is_refused_unless_explicitly_allowed` |
| No surprise redirects | `RedirectPolicy::None` by default (a 3xx is `Error::Redirected`); `Limited(n)` re-checks host and scheme on every hop | `redirects_are_refused_by_default`, `limited_redirects_follow_allowed_hops_only` |
| Size cap | `max_bytes` checked against `Content-Length` before storing anything, and against the running byte count when no length is declared | `oversize_downloads_are_refused`, `the_byte_limit_applies_when_no_length_was_declared` |
| Declared length is the truth | More bytes than `Content-Length` is `SizeMismatch`; fewer, or a broken connection, is `Truncated` | `a_truncated_body_is_rejected_and_cleaned_up`, `more_bytes_than_declared_...` |
| Timeouts | `connect_timeout` and `read_timeout` (longest silence between reads). No overall limit, so slow but live transfers finish | `a_stalled_server_ends_in_a_timeout_and_leaves_nothing` |
| Blocked extensions | Programs, scripts and shortcuts (`exe`, `msi`, `bat`, `ps1`, `lnk`, `scr`, ...) give `Error::Blocked` before any request; the list is configurable | `blocked_extensions_are_refused_before_any_request` |
| Safe file names | Last path component only, control and bidirectional characters removed, `< > : " \| ? *` replaced, edge dots and spaces trimmed, Windows device names (`CON`, `COM1`, ...) prefixed, capped at 200 UTF-8 bytes keeping the extension | `names::tests::sanitising_table`, `names_cannot_escape_the_destination_directory` |
| Partial file next to the target | `<name>.part`, created with `create_new` so an existing file is never reused; numbered if the name is taken | `a_temp_file_left_by_a_crash_does_not_block_the_next_try` |
| Cleaned up on every exit | A `Drop` guard removes the `.part` file on error, cancel, panic in a callback, or when the future is dropped. During the final move the guard travels with the move, so a drop at that moment cannot delete the file underneath it | `dropping_the_transfer_removes_the_partial_file`, `dropping_the_transfer_during_the_final_step_leaves_a_whole_file_or_nothing`, the `leaves nothing` tests |
| Bytes really on disk | `flush`, `sync_all`, then the file length on disk must equal the bytes received (catches a background write that failed late) | `a_last_write_that_fails_late_saves_nothing`, `a_file_shorter_on_disk_than_received_saves_nothing` |
| Never overwrites | The final name is claimed with a hard link, which fails if the name exists; a taken name becomes `name (1).ext` (`Collision::Number`) or `Error::Exists` (`Collision::Fail`). Without hard links the name is reserved with `create_new` instead | `an_existing_file_is_never_overwritten`, `the_fail_policy_refuses_before_any_request`, `a_file_system_without_hard_links_falls_back_to_the_reservation` |
| No placeholder under the final name | The finished `.part` file is linked to its final name in one step, and only then is the `.part` name removed, so no empty file ever holds the final name and a crash cannot leave one that looks like a finished download | `no_placeholder_holds_the_final_name_before_the_file_is_complete` |
| Bounded retries | Removing the `.part` name and, in the fallback, the rename are retried a bounded number of times (virus scanners can hold a fresh file) | `renames_are_retried_a_bounded_number_of_times` |
| Cancellation | `CancelToken` ends even a stalled read at once, closes the connection and removes the partial file | `cancelling_mid_stream_leaves_no_files_and_closes_the_connection` |
| No URL in errors | Errors are categories, never `reqwest` errors, so a bearer-token URL cannot leak into logs | `error_messages_never_leak_the_url` |
| Mark of the web (Windows) | `Zone.Identifier` is written so SmartScreen and Protected View still apply; no-op elsewhere | `saved_files_are_marked_as_downloaded_when_asked` |

## What happens in which order

`download` decides everything it can without the network first, and returns before any
request when one check fails:

1. URL: scheme, credentials, host allow-list (`InvalidUrl`, `SchemeNotAllowed`, `HostNotAllowed`)
2. File name: sanitised, must not be empty (`InvalidName`)
3. Extension: not in `blocked_extensions` (`Blocked`)
4. Destination directory exists, and the name is free under `Collision::Fail` (`Io`, `Exists`)
5. Cancel token not already fired (`Cancelled`)

Then it sends the request and checks the answer: a redirect is `Redirected`, anything but
`200 OK` is `Status`, a declared length over `max_bytes` is `TooLarge`. Only now is the
`.part` file created, so a refused response leaves nothing to clean up.

## Options

`Options::for_hosts([...])` gives conservative defaults; every field is public.

| Field | Default | Meaning |
| --- | --- | --- |
| `allowed_hosts` | empty (refuse all) | Exact host names the URL may use |
| `https_only` | `true` | Refuse `http://` |
| `max_bytes` | 1 GiB | Largest accepted body |
| `connect_timeout` | 15 s | Time to establish a connection |
| `read_timeout` | 60 s | Longest silence between two reads |
| `redirects` | `RedirectPolicy::None` | Or `Limited(n)`; each hop is re-checked |
| `blocked_extensions` | `DEFAULT_BLOCKED_EXTENSIONS` | Case-insensitive, with or without the dot; empty disables |
| `collision` | `Collision::Number` | Or `Collision::Fail` |
| `progress_interval` | 250 ms | Minimum time between progress callbacks |
| `rename_attempts` / `rename_pause` | 5 / 200 ms | Bounded retries for the final rename |
| `mark_of_the_web` | `true` | Windows `Zone.Identifier` tag |
| `user_agent` | `safe-download/<version>` | Must be a valid header value |

Progress is reported as `(bytes_received, total)` where `total` is `None` when the server
sent no `Content-Length`. The first call is `(0, total)`, values only grow, and the last call
carries the final size.

## Notes and limits

- The file name is whatever the caller passes. `Content-Disposition` is ignored on purpose.
- HTTP/1.1 only, no resume, no proxy auto-detection: the dependency set is kept to
  `reqwest` (with `stream` and `rustls-no-provider`), `rustls` (with `ring`) and `tokio`.
  If your application has not installed a rustls crypto provider, the first download installs `ring`.
- `Drop` cannot run when the process aborts or the machine loses power. A leftover `.part`
  file is harmless: later downloads use a numbered temp name and never reuse it. If that
  happens right after the final name was linked, the `.part` name is a second name for the
  same, complete file.
- Dropping the `download` future (for example through `tokio::time::timeout`) while the final
  move is running does not stop the move, which finishes on a blocking thread. The complete
  file then ends up under its final name, never a partial or empty one, although no `Saved`
  is returned. A drop at any earlier point leaves nothing behind.
- File systems without hard links (FAT and exFAT, some network shares) take a fallback: an
  empty file is created with `create_new` under the final name and the finished file is
  renamed over it. A crash or power loss inside that short window leaves a 0-byte file under
  the real name, which looks like a finished download and makes the next one `name (1).ext`.
  Another process that deletes the reservation and creates the same name inside the window is
  not defended against either.
- The connect timeout is configuration only; it is not exercised by the tests (it would need
  an address that silently drops packets).

## Develop

```sh
cargo test -p safe-download
```

The integration tests start a throwaway HTTP server on `127.0.0.1` built from `std::net`
only, so they need no network and no extra crates. Cargo's `target/` directory is ignored by
the repository root `.gitignore` (and by `crates/safe-download/.gitignore`).

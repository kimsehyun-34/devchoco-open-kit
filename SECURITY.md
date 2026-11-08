# Security Policy

## Supported versions

Security fixes are made on the latest release and on `main`. Older versions are
not patched.

## Reporting a vulnerability

Please **do not open a public issue** for a security problem.

Report it privately with GitHub security advisories:

1. Open the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Describe the problem and submit the advisory.

Useful details to include:

- Which part is affected (`packages/channel-icons` or `crates/safe-download`)
  and the version or commit.
- Steps or a minimal example that reproduces the problem.
- What an attacker could achieve, for example writing outside the intended
  download directory or injecting markup.
- Your operating system and toolchain versions, if relevant.

## What to expect

This is a volunteer-run project, so the following are goals, not guarantees:

- Acknowledgement of your report within about 7 days.
- An initial assessment and a plan within about 14 days.
- A fix and a coordinated disclosure through the advisory once a patched
  release is available. Reporters are credited unless they ask not to be.

## Scope

In scope: vulnerabilities in the code in this repository, such as path
traversal or unsafe file-name handling in `crates/safe-download`, and
cross-site scripting or unsafe markup handling in `packages/channel-icons`.

Out of scope: vulnerabilities in third-party dependencies (report those
upstream), and issues that require a compromised development machine.

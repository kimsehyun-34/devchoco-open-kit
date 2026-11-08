# devchoco-open-kit

[![CI](https://github.com/kimsehyun-34/devchoco-open-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/kimsehyun-34/devchoco-open-kit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Small, well-tested building blocks for desktop and web apps, kept in one
monorepo. Each part is independent, has no application-specific code, and can
be used on its own.

| Part | Language | What it does |
| --- | --- | --- |
| [`packages/channel-icons`](packages/channel-icons) | TypeScript / React | Accessible channel icon tile and icon/color picker: inline SVG glyphs, automatic glyph contrast, native radio groups. |
| [`crates/safe-download`](crates/safe-download) | Rust | Streams a URL into a file under an explicit policy (HTTPS and host allow-list, size and time limits, sanitised file names, blocked executable types). Never overwrites a file or leaves a partial one behind; supports progress callbacks and cancellation. |

## Install

Nothing is published to npm or crates.io yet, so use the repository directly. (The
per-package READMEs show the registry commands you would use after a release.)

```sh
git clone https://github.com/kimsehyun-34/devchoco-open-kit.git
cd devchoco-open-kit
pnpm install
```

- **channel-icons**: build it with `pnpm --filter channel-icons build`, then
  depend on the folder (`"channel-icons": "file:../devchoco-open-kit/packages/channel-icons"`)
  or run `pnpm pack` inside the package.
- **safe-download**: add it to a Rust project as a git dependency, or as a path
  dependency (`safe-download = { path = "../devchoco-open-kit/crates/safe-download" }`).

## Usage

Each part documents its own API, with examples, in its README:

- [`packages/channel-icons/README.md`](packages/channel-icons/README.md)
- [`crates/safe-download/README.md`](crates/safe-download/README.md)

## Repository layout

```text
.
├── packages/
│   └── channel-icons/     TypeScript package (Vitest tests)
├── crates/
│   └── safe-download/     Rust crate (cargo tests)
├── .github/               CI, issue and PR templates, Dependabot, CODEOWNERS
├── package.json           pnpm workspace root (scripts only, private)
├── pnpm-workspace.yaml    pnpm workspace definition
├── Cargo.toml             Cargo workspace definition
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
└── LICENSE
```

## Development

Requirements: Node.js 20.19 or newer (CI runs 22), pnpm 10, and a stable Rust
toolchain with `rustfmt` and `clippy`.

```sh
# TypeScript packages
pnpm install
pnpm -r build
pnpm -r test

# Rust crates (run from the repository root)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on Ubuntu and Windows for every push and pull
request. See [CONTRIBUTING.md](CONTRIBUTING.md) for the branch and pull request
flow.

## Security

Please report vulnerabilities privately; see [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)

---

## 한국어 요약

`devchoco-open-kit`은 데스크톱·웹 앱에서 재사용할 수 있는 작고 잘 테스트된 구성 요소를 한 저장소에 모은 프로젝트입니다.

- `packages/channel-icons`: 접근성을 고려한 React용 채널 아이콘 타일과 아이콘/색상 선택기입니다.
- `crates/safe-download`: URL의 내용을 파일로 안전하게 내려받는 Rust 크레이트입니다. HTTPS·허용 호스트 정책, 크기·시간 제한, 파일 이름 정리, 실행 파일 형식 차단을 지원하며, 기존 파일을 덮어쓰거나 불완전한 파일을 남기지 않고 진행률 콜백과 취소를 제공합니다.

아직 npm이나 crates.io에 배포하지 않았으므로 저장소를 직접 받아 사용합니다. 테스트는 `pnpm install` 후 `pnpm -r test`, Rust는 저장소 루트에서 `cargo test --workspace`로 실행합니다. 기여 방법은 [CONTRIBUTING.md](CONTRIBUTING.md), 보안 제보는 [SECURITY.md](SECURITY.md)를 참고하세요. 라이선스는 MIT입니다.

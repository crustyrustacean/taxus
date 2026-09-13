+++
title = "Install"
description = "Install a prebuilt taxus binary for your platform, or build from source."
+++

The fastest way to get taxus is a prebuilt binary from the
[latest release](https://github.com/crustyrustacean/taxus/releases/latest).
Six platforms are built for every release: macOS, Linux and Windows,
ARM64 and x86-64.

## One-line installers

{{% box(class="install-hint") %}}
The installers place `taxus` in `~/.cargo/bin` — the same directory as
`cargo` — so it is on your `PATH` if Rust is installed. Building from
source needs nothing but the script.
{{% /box %}}

**macOS / Linux** (shell):

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.sh | sh
```

**Windows** (PowerShell):

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.ps1 | iex"
```

Verify the install:

```sh
taxus --version
```

## Manual download

Grab the archive for your platform from the
[releases page](https://github.com/crustyrustacean/taxus/releases/latest)
and put `taxus` (or `taxus.exe`) somewhere on your `PATH`. Every
archive ships with a `.sha256` checksum.

| Platform | Archive |
|----------|---------|
| macOS (Apple Silicon) | `taxus-aarch64-apple-darwin.tar.xz` |
| macOS (Intel) | `taxus-x86_64-apple-darwin.tar.xz` |
| Linux (ARM64) | `taxus-aarch64-unknown-linux-gnu.tar.xz` |
| Linux (x86-64) | `taxus-x86_64-unknown-linux-gnu.tar.xz` |
| Windows (ARM64) | `taxus-aarch64-pc-windows-msvc.zip` |
| Windows (x86-64) | `taxus-x86_64-pc-windows-msvc.zip` |

## Build from source

Taxus is a Rust workspace; any recent stable toolchain builds it:

```sh
git clone https://github.com/crustyrustacean/taxus.git
cd taxus
cargo build --release
# the binary is target/release/taxus
```

The build compiles the WASM islands client and embeds it in the
binary — no separate step, no Node toolchain.

## Next steps

- [Get started](/authoring/) authoring content
- Read the [deployment guide](https://crustyrustacean.github.io/taxus/deployment.html) for hosting the built site

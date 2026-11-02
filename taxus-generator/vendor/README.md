# Bundled TypeScript compilers

This directory carries the unmodified official TypeScript 7.0.2 native release
archives from:

https://github.com/microsoft/TypeScript/releases/tag/v7.0.2

Taxus selects one archive at compile time for each supported release target:

| Target family | Archive |
|---|---|
| macOS arm64 | `typescript-darwin-arm64.tgz` |
| macOS x64 | `typescript-darwin-x64.tgz` |
| Linux arm64 | `typescript-linux-arm64.tgz` |
| Linux x64 | `typescript-linux-x64.tgz` |
| Windows arm64 | `typescript-win32-arm64.tgz` |
| Windows x64 | `typescript-win32-x64.tgz` |

The archives are Apache-2.0 licensed. `typescript-LICENSE.txt` and
`typescript-NOTICE.txt` are extracted from the same release package. The
archives are compiled into Taxus only when the `typescript` Cargo feature is
enabled; ordinary Cargo builds contain none of them. Taxus's release
configuration enables the feature for published binaries. Each embedded archive
is verified against its pinned SHA-256 digest at runtime.

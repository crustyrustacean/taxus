# Bundled TypeScript compiler (proof of concept)

This directory carries the unmodified official TypeScript 7.0.2 Linux x64
release archive from:

https://github.com/microsoft/TypeScript/releases/tag/v7.0.2

Archive: `typescript-linux-x64-v7.0.2.tgz`
SHA-256: `7ecad6f67377e831856367ab062ef394f21506a611405bf8ac0ff039348637d3`

The archive is Apache-2.0 licensed. `typescript-LICENSE.txt` and
`typescript-NOTICE.txt` are extracted from the same release package. The
current PoC embeds this Linux-only archive in every Taxus build; production
support needs target-specific native compiler packages or a different
packaging strategy.

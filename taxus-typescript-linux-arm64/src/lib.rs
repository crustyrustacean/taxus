//! Bundled TypeScript compiler data for the linux-arm64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-linux-arm64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "c83d931ac9dd7549cde6e71246aa9d6a9812843023df3e277fe3b5dcf41dd0ea";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-linux-arm64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc";

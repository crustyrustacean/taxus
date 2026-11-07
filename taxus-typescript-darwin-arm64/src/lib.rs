//! Bundled TypeScript compiler data for the darwin-arm64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-darwin-arm64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "902e2fe1cf0799198ef902c6b8c310a450fef629a6baba41d45641ef75c04ebd";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-darwin-arm64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc";

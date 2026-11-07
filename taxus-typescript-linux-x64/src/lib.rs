//! Bundled TypeScript compiler data for the linux-x64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-linux-x64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "7ecad6f67377e831856367ab062ef394f21506a611405bf8ac0ff039348637d3";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-linux-x64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc";

//! Bundled TypeScript compiler data for the darwin-x64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-darwin-x64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "eba158cb54050f723d5ff781438f33de5640054440bb4f2bd170cfe9bc2eb551";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-darwin-x64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc";

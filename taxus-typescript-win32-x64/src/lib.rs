//! Bundled TypeScript compiler data for the win32-x64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-win32-x64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "61fc4e141d2bc687db580e71bbfa63b9c209f0310645d82ca1b457eb3a24fd19";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-win32-x64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc.exe";

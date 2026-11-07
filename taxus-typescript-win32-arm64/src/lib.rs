//! Bundled TypeScript compiler data for the win32-arm64 target.

/// The compressed TypeScript compiler archive for this target.
pub static ARCHIVE: &[u8] = include_bytes!("../vendor/typescript-win32-arm64.tgz");

/// SHA-256 digest of `ARCHIVE`.
pub const SHA256: &str = "0a73534e6ee50cdbb2a29ac48657ca0ad13cf0f424cf63808e4df7baeb87b8be";

/// Stable cache directory name for the extracted compiler.
pub const CACHE_NAME: &str = "taxus-typescript-7.0.2-win32-arm64";

/// Path to the compiler executable within the extracted archive.
pub const COMPILER_PATH: &str = "package/lib/tsc.exe";

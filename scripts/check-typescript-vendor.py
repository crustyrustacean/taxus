#!/usr/bin/env python3
"""Validate the vendored TypeScript compiler archives and their source hashes.

Run this after downloading or replacing a TypeScript release. The archive names,
SHA-256 values, and compiler paths here are deliberately duplicated from the
Rust target mapping so a compiler update fails loudly until both sides agree.
"""

from __future__ import annotations

import hashlib
import sys
import tarfile
from pathlib import Path

VERSION = "7.0.2"
ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_CONFIG = ROOT / "dist-workspace.toml"

ARCHIVES = {
    "darwin-arm64": (
        "typescript-darwin-arm64.tgz",
        "902e2fe1cf0799198ef902c6b8c310a450fef629a6baba41d45641ef75c04ebd",
        "package/lib/tsc",
    ),
    "darwin-x64": (
        "typescript-darwin-x64.tgz",
        "eba158cb54050f723d5ff781438f33de5640054440bb4f2bd170cfe9bc2eb551",
        "package/lib/tsc",
    ),
    "linux-arm64": (
        "typescript-linux-arm64.tgz",
        "c83d931ac9dd7549cde6e71246aa9d6a9812843023df3e277fe3b5dcf41dd0ea",
        "package/lib/tsc",
    ),
    "linux-x64": (
        "typescript-linux-x64.tgz",
        "7ecad6f67377e831856367ab062ef394f21506a611405bf8ac0ff039348637d3",
        "package/lib/tsc",
    ),
    "win32-arm64": (
        "typescript-win32-arm64.tgz",
        "0a73534e6ee50cdbb2a29ac48657ca0ad13cf0f424cf63808e4df7baeb87b8be",
        "package/lib/tsc.exe",
    ),
    "win32-x64": (
        "typescript-win32-x64.tgz",
        "61fc4e141d2bc687db580e71bbfa63b9c209f0310645d82ca1b457eb3a24fd19",
        "package/lib/tsc.exe",
    ),
}


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    dist_config = WORKSPACE_CONFIG.read_text()
    if 'features = ["typescript"]' not in dist_config:
        fail("dist-workspace.toml does not enable the TypeScript feature for releases")
    for target in (
        "aarch64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "aarch64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
    ):
        if target not in dist_config:
            fail(f"dist-workspace.toml does not publish {target}")
    for target, (filename, expected_hash, compiler) in ARCHIVES.items():
        package = ROOT / f"taxus-typescript-{target}"
        archive = package / "vendor" / filename
        rust_source = package / "src" / "lib.rs"
        if not archive.is_file():
            fail(f"{target}: missing {archive}")

        actual_hash = sha256(archive)
        if actual_hash != expected_hash:
            fail(f"{target}: SHA-256 {actual_hash} does not match {expected_hash}")
        if f'"{expected_hash}"' not in rust_source.read_text():
            fail(f"{target}: SHA-256 is not pinned in {rust_source}")

        try:
            with tarfile.open(archive, "r:gz") as tar:
                names = {member.name for member in tar.getmembers()}
        except tarfile.TarError as error:
            fail(f"{target}: cannot read {archive}: {error}")

        if compiler not in names:
            fail(f"{target}: archive does not contain {compiler}")
        if "package/lib/lib.dom.d.ts" not in names:
            fail(f"{target}: archive does not contain package/lib/lib.dom.d.ts")

        print(f"ok: {target} {filename} ({actual_hash})")

    print(f"all TypeScript {VERSION} compiler archives are valid")


if __name__ == "__main__":
    main()

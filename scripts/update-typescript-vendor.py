#!/usr/bin/env python3
"""Download and pin an official TypeScript native compiler release.

Usage:
    python3 scripts/update-typescript-vendor.py --version 7.0.3

The script uses only Python's standard library. It updates the six target
archives, license/notice files, Rust SHA-256/cache metadata, and the archive
checker. Review the resulting diff before committing.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import re
import tarfile
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
VENDOR = ROOT / "taxus-generator" / "vendor"
RUST_SOURCE = ROOT / "taxus-generator" / "src" / "assets" / "typescript.rs"
CHECKER = ROOT / "scripts" / "check-typescript-vendor.py"
VENDOR_README = VENDOR / "README.md"

TARGETS = (
    "typescript-darwin-arm64.tgz",
    "typescript-darwin-x64.tgz",
    "typescript-linux-arm64.tgz",
    "typescript-linux-x64.tgz",
    "typescript-win32-arm64.tgz",
    "typescript-win32-x64.tgz",
)


def fetch_json(url: str):
    request = Request(url, headers={"User-Agent": "taxus-typescript-updater"})
    with urlopen(request) as response:
        return json.load(response)


def fetch_bytes(url: str) -> bytes:
    request = Request(url, headers={"User-Agent": "taxus-typescript-updater"})
    with urlopen(request) as response:
        return response.read()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"error: {message}")


def replace_all(path: Path, old: str, new: str) -> None:
    text = path.read_text()
    if old not in text:
        fail(f"{path}: could not find {old!r}")
    path.write_text(text.replace(old, new))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True, help="TypeScript release, e.g. 7.0.3")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail(f"invalid version: {args.version}")

    release = fetch_json(f"https://api.github.com/repos/microsoft/TypeScript/releases/tags/v{version}")
    assets = {asset["name"]: asset for asset in release["assets"]}

    hashes = []
    for filename in TARGETS:
        asset = assets.get(filename)
        if asset is None:
            fail(f"release v{version} has no {filename}")
        digest = asset.get("digest", "")
        if not digest.startswith("sha256:"):
            fail(f"release asset {filename} has no SHA-256 digest")
        expected = digest.removeprefix("sha256:")
        data = fetch_bytes(asset["browser_download_url"])
        actual = sha256(data)
        if actual != expected:
            fail(f"downloaded {filename} has SHA-256 {actual}, expected {expected}")
        (VENDOR / filename).write_bytes(data)
        hashes.append(expected)
        print(f"downloaded {filename} ({len(data)} bytes, {actual})")

    generic = assets.get("typescript.tgz")
    if generic is None:
        fail(f"release v{version} has no generic typescript.tgz asset")
    with tarfile.open(fileobj=io.BytesIO(fetch_bytes(generic["browser_download_url"])), mode="r:gz") as archive:
        members = {member.name: member for member in archive.getmembers()}
        for source, destination in {
            "package/LICENSE": VENDOR / "typescript-LICENSE.txt",
            "package/NOTICE.txt": VENDOR / "typescript-NOTICE.txt",
        }.items():
            member = members.get(source)
            if member is None:
                fail(f"generic archive has no {source}")
            extracted = archive.extractfile(member)
            if extracted is None:
                fail(f"could not extract {source}")
            destination.write_bytes(extracted.read())
            print(f"updated {destination.relative_to(ROOT)}")

    rust = RUST_SOURCE.read_text()
    rust_hashes = list(re.finditer(r'(const BUNDLED_SHA256: &str = ")([0-9a-f]{64})(";)', rust))
    if len(rust_hashes) != len(hashes):
        fail(f"expected {len(hashes)} Rust SHA-256 constants, found {len(rust_hashes)}")
    for match, digest in reversed(list(zip(rust_hashes, hashes))):
        start, end = match.span(2)
        rust = rust[:start] + digest + rust[end:]
    rust = re.sub(r"taxus-typescript-[0-9]+\.[0-9]+\.[0-9]+-", f"taxus-typescript-{version}-", rust)
    RUST_SOURCE.write_text(rust)

    checker = CHECKER.read_text()
    checker = re.sub(r'VERSION = "[^"]+"', f'VERSION = "{version}"', checker)
    checker_hashes = list(re.finditer(r'("[0-9a-f]{64}")', checker))
    if len(checker_hashes) < len(hashes):
        fail("could not find all checker SHA-256 values")
    for match, digest in reversed(list(zip(checker_hashes[: len(hashes)], hashes))):
        start, end = match.span(1)
        checker = checker[:start] + f'"{digest}"' + checker[end:]
    CHECKER.write_text(checker)

    readme = VENDOR_README.read_text()
    readme = re.sub(r"TypeScript 7\.0\.2", f"TypeScript {version}", readme)
    VENDOR_README.write_text(readme)
    print(f"updated TypeScript vendor metadata to {version}")
    print("run scripts/check-typescript-vendor.py and review the diff before committing")


if __name__ == "__main__":
    main()

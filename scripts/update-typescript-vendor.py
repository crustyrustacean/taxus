#!/usr/bin/env python3
"""Download and pin official target-specific TypeScript compiler releases.

Usage:
    python3 scripts/update-typescript-vendor.py --version 7.0.3

Each platform archive lives in its own publishable support crate so no single
crates.io package exceeds the upload limit.
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
CHECKER = ROOT / "scripts" / "check-typescript-vendor.py"
TARGETS = (
    "darwin-arm64",
    "darwin-x64",
    "linux-arm64",
    "linux-x64",
    "win32-arm64",
    "win32-x64",
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True, help="TypeScript release, e.g. 7.0.3")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail(f"invalid version: {args.version}")

    release = fetch_json(f"https://api.github.com/repos/microsoft/TypeScript/releases/tags/v{version}")
    assets = {asset["name"]: asset for asset in release["assets"]}
    hashes: list[str] = []

    for target in TARGETS:
        filename = f"typescript-{target}.tgz"
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
        vendor = ROOT / f"taxus-typescript-{target}" / "vendor"
        (vendor / filename).write_bytes(data)
        hashes.append(expected)
        print(f"downloaded {filename} ({len(data)} bytes, {actual})")

    generic = assets.get("typescript.tgz")
    if generic is None:
        fail(f"release v{version} has no generic typescript.tgz asset")
    with tarfile.open(fileobj=io.BytesIO(fetch_bytes(generic["browser_download_url"])), mode="r:gz") as archive:
        members = {member.name: member for member in archive.getmembers()}
        for source, filename in {
            "package/LICENSE": "typescript-LICENSE.txt",
            "package/NOTICE.txt": "typescript-NOTICE.txt",
        }.items():
            member = members.get(source)
            if member is None:
                fail(f"generic archive has no {source}")
            extracted = archive.extractfile(member)
            if extracted is None:
                fail(f"could not extract {source}")
            data = extracted.read()
            for target in TARGETS:
                destination = ROOT / f"taxus-typescript-{target}" / "vendor" / filename
                destination.write_bytes(data)
                print(f"updated {destination.relative_to(ROOT)}")

    for target, digest in zip(TARGETS, hashes):
        source = ROOT / f"taxus-typescript-{target}" / "src" / "lib.rs"
        text = source.read_text()
        text = re.sub(r'pub const SHA256: &str = "[0-9a-f]{64}";', f'pub const SHA256: &str = "{digest}";', text)
        text = re.sub(r"taxus-typescript-7\.0\.2-", f"taxus-typescript-{version}-", text)
        source.write_text(text)

    checker = CHECKER.read_text()
    checker = re.sub(r'VERSION = "[^"]+"', f'VERSION = "{version}"', checker)
    checker_hashes = list(re.finditer(r'("[0-9a-f]{64}")', checker))
    if len(checker_hashes) < len(hashes):
        fail("could not find all checker SHA-256 values")
    for match, digest in reversed(list(zip(checker_hashes[: len(hashes)], hashes))):
        start, end = match.span(1)
        checker = checker[:start] + f'"{digest}"' + checker[end:]
    CHECKER.write_text(checker)
    print(f"updated TypeScript vendor metadata to {version}")
    print("run scripts/check-typescript-vendor.py and review the diff before committing")


if __name__ == "__main__":
    main()

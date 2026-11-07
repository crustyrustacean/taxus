#!/usr/bin/env python3
"""Compile a consumer against the packaged taxus-ssg artifact.

The support crates are not published yet, so cargo package receives temporary
local patches for taxus-domain and taxus-common. The consumer itself depends
only on the extracted .crate and uses the public `taxus` crate name.
"""

from __future__ import annotations

import json
import subprocess
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TARGET_PACKAGE = ROOT / "target" / "package"


def run(*args: str, cwd: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, check=True, text=True)


def package_version() -> str:
    metadata = subprocess.run(
        "cargo metadata --format-version 1 --no-deps".split(),
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    packages = json.loads(metadata.stdout)["packages"]
    return next(package["version"] for package in packages if package["name"] == "taxus-ssg")


def main() -> int:
    version = package_version()
    crate_name = f"taxus-ssg-{version}.crate"
    archive = TARGET_PACKAGE / crate_name

    run(
        "cargo",
        "package",
        "-p",
        "taxus-ssg",
        "--allow-dirty",
        "--no-verify",
        "--config",
        f'patch.crates-io.taxus-common.path="{ROOT / "taxus-common"}"',
        "--config",
        f'patch.crates-io.taxus-domain.path="{ROOT / "taxus-domain"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-darwin-arm64.path="{ROOT / "taxus-typescript-darwin-arm64"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-darwin-x64.path="{ROOT / "taxus-typescript-darwin-x64"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-linux-arm64.path="{ROOT / "taxus-typescript-linux-arm64"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-linux-x64.path="{ROOT / "taxus-typescript-linux-x64"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-win32-arm64.path="{ROOT / "taxus-typescript-win32-arm64"}"',
        "--config",
        f'patch.crates-io.taxus-typescript-win32-x64.path="{ROOT / "taxus-typescript-win32-x64"}"',
    )

    if not archive.is_file():
        raise RuntimeError(f"cargo package did not create {archive}")

    with tarfile.open(archive, "r:gz") as package:
        names = set(package.getnames())
        prefix = f"taxus-ssg-{version}/"
        required = {
            f"{prefix}Cargo.toml",
            f"{prefix}README.md",
            f"{prefix}build.rs",
            f"{prefix}src/lib.rs",
        }
        missing = required - names
        if missing:
            raise RuntimeError(f"packaged crate is missing: {sorted(missing)}")

        forbidden_fragments = (
            "/src/bin/",
            "/src/init/",
            "/src/serve",
            "/src/telemetry.rs",
            "/taxus-cli/",
        )
        forbidden = sorted(
            name for name in names if any(fragment in name for fragment in forbidden_fragments)
        )
        if forbidden:
            raise RuntimeError(f"product-only files leaked into taxus-ssg: {forbidden}")

        with tempfile.TemporaryDirectory(prefix="taxus-packaged-consumer-") as temp:
            temp_root = Path(temp)
            package.extractall(temp_root)
            consumer = temp_root / "consumer"
            (consumer / "src").mkdir(parents=True)
            patch_crates = [
                "taxus-common",
                "taxus-domain",
                "taxus-typescript-darwin-arm64",
                "taxus-typescript-darwin-x64",
                "taxus-typescript-linux-arm64",
                "taxus-typescript-linux-x64",
                "taxus-typescript-win32-arm64",
                "taxus-typescript-win32-x64",
            ]
            manifest = f"""[package]
name = "packaged-taxus-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
taxus = {{ package = "taxus-ssg", path = "../taxus-ssg-{version}", default-features = false }}

[patch.crates-io]
"""
            manifest += "".join(
                f'{crate} = {{ path = "{ROOT / crate}" }}\n'
                for crate in patch_crates
            )
            (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8")
            (consumer / "src/main.rs").write_text(
                """use taxus::{SiteBuilder, SiteConfig};\n\nfn main() {\n    let _ = SiteBuilder::new as fn(SiteConfig) -> SiteBuilder;\n}\n""",
                encoding="utf-8",
            )
            run(
                "cargo",
                "check",
                "--manifest-path",
                str(consumer / "Cargo.toml"),
                "--no-default-features",
                cwd=consumer,
            )

    print(f"validated packaged consumer against {archive}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error

#!/usr/bin/env python3
"""Run a small generated TypeScript site in headless Firefox via Marionette."""

from __future__ import annotations

import argparse
import json
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path
from urllib.parse import urlparse


class Marionette:
    def __init__(self, host: str = "127.0.0.1", port: int = 2828):
        self.socket = socket.create_connection((host, port), timeout=5)
        self.socket.settimeout(10)
        self.next_id = 1
        self._read_packet()  # greeting

    def _read_exact(self, size: int) -> bytes:
        data = b""
        while len(data) < size:
            chunk = self.socket.recv(size - len(data))
            if not chunk:
                raise RuntimeError("Firefox closed the Marionette connection")
            data += chunk
        return data

    def _read_packet(self):
        prefix = b""
        while b":" not in prefix:
            prefix += self.socket.recv(1)
        size = int(prefix[:-1])
        return json.loads(self._read_exact(size))

    def command(self, name: str, parameters: dict):
        command_id = self.next_id
        self.next_id += 1
        payload = json.dumps(
            [0, command_id, name, parameters], separators=(",", ":")
        ).encode()
        self.socket.sendall(str(len(payload)).encode() + b":" + payload)
        response = self._read_packet()
        if response[1] != command_id:
            raise RuntimeError(f"unexpected Marionette response: {response}")
        if response[2] is not None:
            raise RuntimeError(f"{name} failed: {response[2]}")
        return response[3]

    def close(self):
        self.socket.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", required=True)
    args = parser.parse_args()

    parsed = urlparse(args.url)
    if parsed.scheme not in {"http", "https"}:
        raise SystemExit("--url must be an http(s) URL")

    firefox = shutil.which("firefox")
    if firefox is None:
        raise SystemExit("Firefox is required for the TypeScript browser smoke test")

    with tempfile.TemporaryDirectory(prefix="taxus-firefox-") as profile:
        process = subprocess.Popen(
            [
                firefox,
                "--headless",
                "--no-remote",
                "--profile",
                profile,
                "--marionette",
                "about:blank",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        driver = None
        try:
            deadline = time.monotonic() + 15
            while driver is None and time.monotonic() < deadline:
                try:
                    driver = Marionette()
                except (ConnectionRefusedError, TimeoutError, OSError):
                    time.sleep(0.25)
            if driver is None:
                raise SystemExit("Firefox did not open its Marionette endpoint")

            driver.command("WebDriver:NewSession", {"capabilities": {"alwaysMatch": {}}})
            driver.command("WebDriver:Navigate", {"url": args.url})
            result = driver.command(
                "WebDriver:ExecuteScript",
                {
                    "script": """
                        return {
                          marker: document.documentElement.dataset.typescriptSmoke,
                          module: Boolean(document.querySelector(
                            'script[type="module"][src="/static/ts/main.js"]'
                          )),
                        };
                    """,
                    "args": [],
                },
            )["value"]
            if result != {"marker": "passed", "module": True}:
                raise SystemExit(f"unexpected browser result: {result!r}")
            print("TypeScript browser smoke test passed")
        finally:
            if driver is not None:
                try:
                    driver.command("WebDriver:DeleteSession", {})
                except (OSError, RuntimeError):
                    pass
                driver.close()
            process.terminate()
            process.wait(timeout=10)


if __name__ == "__main__":
    main()

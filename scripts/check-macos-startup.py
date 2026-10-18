"""Check that the actual macOS app bundle starts and stays running.

The internal executable must use an ASCII name: the previous Chinese filename
crashed in NSBundle's WebKit version lookup before the app could create a window.
Run only against a trusted, locally built app. The test closes its own process.
"""

import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import time


def main():
    if sys.platform != "darwin":
        raise SystemExit("This startup check requires macOS.")
    app = Path(sys.argv[1]).resolve()
    with (app / "Contents/Info.plist").open("rb") as file:
        info = plistlib.load(file)
    name = info["CFBundleExecutable"]
    if not name.isascii():
        raise SystemExit("The macOS internal executable name must be ASCII.")
    executable = app / "Contents/MacOS" / name
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen(
            [str(executable)],
            stdout=stdout,
            stderr=stderr,
            env=dict(os.environ, RUST_BACKTRACE="full"),
        )
        try:
            deadline = time.monotonic() + 30
            while process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.25)
            if process.poll() is not None:
                stderr.seek(0)
                print(stderr.read().decode("utf-8", errors="replace"), file=sys.stderr)
                raise SystemExit(f"App exited during startup: {process.returncode}")
            print(f"PASS: {app.name} ({name}) stayed running for 30 seconds.")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Feed real SGR mouse reports to the mouse example for the VHS recording.

Run after `cargo build --example mouse`. The child inherits the recording terminal size;
stdout is its unmodified terminal output, recorded by docs/tapes/mouse.tape.
"""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import sys
import termios
import time


def mouse(code, x, y, release=False):
    return f"\x1b[<{code};{x};{y}{'m' if release else 'M'}".encode()


def main():
    binary = Path(__file__).resolve().parents[2] / "target/debug/examples/mouse"
    if not binary.is_file():
        sys.exit("Build first: cargo build --example mouse")
    size = fcntl.ioctl(sys.stdin.fileno(), termios.TIOCGWINSZ, bytes(8))
    rows, columns, _, _ = struct.unpack("HHHH", size)
    pid, master = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, size)
        os.execv(str(binary), [str(binary)])
    # Coordinates are 1-based in SGR reports. These interact with the actual app.
    events = [
        (1.0, mouse(35, 4, 4)),       # hover Inc
        (2.0, mouse(0, 4, 4)),
        (2.1, mouse(0, 4, 4, True)),  # press Inc
        (3.0, mouse(0, 33, 8)),      # grab divider
        (3.3, mouse(32, 38, 8)),
        (3.6, mouse(32, 43, 8)),
        (3.9, mouse(0, 43, 8, True)),
        (5.0, mouse(0, 8, 14)),      # release-to-click zone
        (5.05, mouse(0, 8, 14, True)),
        (5.15, mouse(0, 8, 14)),
        (5.2, mouse(0, 8, 14, True)),
        (6.5, mouse(0, 29, 4)),      # open modal
        (6.6, mouse(0, 29, 4, True)),
        (7.3, mouse(0, columns // 4 + 3, rows // 2 - 1)),
        (7.4, mouse(0, columns // 4 + 3, rows // 2 - 1, True)),
        (8.0, mouse(0, 5, 18)),      # dismiss through scrim
        (8.1, mouse(0, 5, 18, True)),
        (9.0, mouse(35, 10, rows - 1)),    # hover zone
        (9.8, mouse(35, 1, 1)),
        (10.0, mouse(35, 10, rows - 1)),
        (13.5, b"q"),
    ]
    started = time.monotonic()
    try:
        while time.monotonic() - started < 18:
            elapsed = time.monotonic() - started
            while events and events[0][0] <= elapsed:
                _, data = events.pop(0)
                os.write(master, data)
            if select.select([master], [], [], 0.02)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError:
                    break
                if not data:
                    break
                if b"\x1b[6n" in data:
                    os.write(master, b"\x1b[1;1R")
                sys.stdout.buffer.write(data)
                sys.stdout.buffer.flush()
        else:
            raise RuntimeError("mouse example did not exit within 18 seconds")
    finally:
        os.close(master)
        # Bound cleanup if the example fails before the scripted quit.
        waited, status = os.waitpid(pid, os.WNOHANG)
        if waited == 0:
            os.kill(pid, signal.SIGTERM)
            _, status = os.waitpid(pid, 0)
    if os.waitstatus_to_exitcode(status) != 0:
        sys.exit("mouse example exited unsuccessfully")


if __name__ == "__main__":
    main()

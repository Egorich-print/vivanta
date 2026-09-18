#!/usr/bin/env python3
"""Upload a binary to the RK3568 vendor U-Boot console via interactive `mm.l`.

The vendor U-Boot has no `loadx/loady/loadb` and no `go`, so the only reliable
path is `mm.l` (interactive memory modify). Timing must respect the NS16550
64-byte FIFO: <=64 words per chunk, >=60 ms per word, sync after each chunk.

Usage:
    ./flash-serial-mm.py IMAGE [--port /dev/cu.wchusbserial110]
                              [--baud 1500000] [--addr 0x20500000]
                              [--resume-bytes N]

After it finishes, from the U-Boot console:
    mtd erase spi-nand0 0x0 0x100000
    mtd write spi-nand0 0x20500000 0x0 0x2B800
    reset
"""
from __future__ import annotations

import argparse
import struct
import sys
import time

import serial  # pyserial

CHUNK = 64          # words per mm.l invocation (FIFO limit)
WORD_DELAY = 0.06   # s per word
CHUNK_START = 0.10  # s after the mm.l command line
CHUNK_END = 0.15    # s after mm.l exit before next chunk


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("--port", default="/dev/cu.wchusbserial110")
    ap.add_argument("--baud", type=int, default=1500000)
    ap.add_argument("--addr", type=lambda s: int(s, 0), default=0x20500000)
    ap.add_argument("--resume-bytes", type=int, default=0,
                    help="skip the first N bytes (resume an interrupted upload)")
    args = ap.parse_args()

    with open(args.image, "rb") as f:
        data = f.read()
    if len(data) % 4:
        data += b"\x00" * (4 - len(data) % 4)
    words = list(struct.unpack(f"<{len(data) // 4}I", data))

    start = (args.resume_bytes // 4) // CHUNK * CHUNK
    total = len(words)
    print(f"image: {len(data)} B / {total} words")
    print(f"resume: word {start} ({(start * 100) // total}%), "
          f"addr 0x{args.addr + start * 4:08x}")

    with serial.Serial(args.port, args.baud, timeout=10) as s:
        time.sleep(1)
        for _ in range(10):
            s.write(b"\n")
            time.sleep(0.3)
        s.reset_input_buffer()

        t0 = time.time()
        for i in range(start, total, CHUNK):
            chunk = words[i:i + CHUNK]
            s.write(f"mm.l 0x{args.addr + i * 4:08x}\n".encode())
            time.sleep(CHUNK_START)
            for v in chunk:
                s.write(f"0x{v:08x}\n".encode())
                time.sleep(WORD_DELAY)
            s.write(b".\n")
            time.sleep(CHUNK_END)
            s.reset_input_buffer()

            if i and i % (CHUNK * 10) == 0:
                done = i - start
                rate = done / (time.time() - t0)
                eta = (total - i) / rate / 60 if rate else 0
                print(f"\r{i * 100 // total}% | {rate:.1f} w/s | ETA {eta:.1f}m",
                      end="", flush=True)

        print(f"\ndone in {(time.time() - t0) / 60:.1f} min")
        s.write(b"\n")
        time.sleep(0.5)
        s.write(f"md.l 0x{args.addr:08x} 4\n".encode())
        time.sleep(0.5)
        print(s.read(4096).decode("utf-8", errors="replace"))

    return 0


if __name__ == "__main__":
    sys.exit(main())

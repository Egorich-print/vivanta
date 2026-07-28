#!/usr/bin/env python3
"""
RK3568 uploader: kernel + DTB + booti. DTB is auto-generated.
Usage: python3 flash_rk3568.py [binary] [port]
"""

import serial, struct, sys, time, os

BIN = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "images/vivanta-rk3568.bin")
PORT = sys.argv[2] if len(sys.argv) > 2 else "/dev/cu.wchusbserial110"
KERNEL_ADDR = 0x20500000
DTB_ADDR = 0x0A100000
BAUD = 115200

# ---------------------------------------------------------------
# Minimal DTB for RK3568
# ---------------------------------------------------------------
def build_dtb():
    sblk = bytearray()
    strs = [""]

    def align4(buf):
        while len(buf) % 4: buf += b'\x00'
        return buf

    def node(name):
        nonlocal sblk
        sblk += struct.pack('>I', 1)
        sblk += name.encode() + b'\x00'
        sblk[:] = align4(sblk)

    def str_ofs(s):
        nonlocal strs
        if s not in strs: strs.append(s)
        return sum(len(x)+1 for x in strs[:strs.index(s)])

    def prop(name, value):
        nonlocal sblk
        v = value if isinstance(value, bytes) else struct.pack('>I' if len(str(value)) <= 4 else '>Q', value)
        sblk += struct.pack('>I', 3)
        sblk += struct.pack('>II', len(v), str_ofs(name))
        sblk += v
        sblk[:] = align4(sblk)

    node("")
    prop("compatible", b"linux,dummy-virt\x00")
    prop("#address-cells", struct.pack('>I', 2))
    prop("#size-cells", struct.pack('>I', 2))
    node("memory")
    prop("device_type", b"memory\x00")
    prop("reg", struct.pack('>QQ', 0x00200000, 0xEFE00000))
    sblk += struct.pack('>I', 2)
    node("chosen")
    prop("bootargs", b"console=ttyS0,115200\x00")
    sblk += struct.pack('>I', 2)
    node("cpus")
    prop("#address-cells", 1)
    prop("#size-cells", 0)
    node("cpu@0")
    prop("device_type", b"cpu\x00")
    prop("compatible", b"arm,cortex-a55\x00")
    prop("reg", 0)
    sblk += struct.pack('>I', 2)
    sblk += struct.pack('>I', 2)
    sblk += struct.pack('>I', 2)
    sblk += struct.pack('>I', 9)

    string_blk = bytearray()
    for s in strs: string_blk += s.encode() + b'\x00'

    mem_rsv = struct.pack('>QQ', 0, 0)
    totalsize = 40 + len(mem_rsv) + len(sblk) + len(string_blk)
    while totalsize % 8: totalsize += 1
    # Pad to at least 4096 bytes so U-Boot's DTB modifications don't hit
    # stale data from previous uploads
    if totalsize < 4096:
        totalsize = 4096

    header = struct.pack('>IIIIIIIIII',
        0xd00dfeed, totalsize,
        40 + len(mem_rsv), 40 + len(mem_rsv) + len(sblk),
        40, 17, 16, 0, len(string_blk), len(sblk))
    dtb = header + mem_rsv + bytes(sblk) + bytes(string_blk)
    while len(dtb) < totalsize: dtb += b'\x00'
    return dtb

# ---------------------------------------------------------------
# Load
# ---------------------------------------------------------------
with open(BIN, "rb") as f: kern = f.read()
while len(kern) % 4: kern += b'\x00'
dtb = build_dtb()
while len(dtb) % 4: dtb += b'\x00'
kern_words, dtb_words = len(kern)//4, len(dtb)//4
print(f"Kernel: {len(kern)}B, DTB: {len(dtb)}B → 0x{KERNEL_ADDR:08X} / 0x{DTB_ADDR:08X}")

# ---------------------------------------------------------------
# Serial
# ---------------------------------------------------------------
ser = serial.Serial(PORT, BAUD, timeout=2)
ser.reset_input_buffer()
time.sleep(0.3)
ser.reset_input_buffer()

# Wait for prompt
print("[1] Waiting for U-Boot ...")
buf = b""
t0 = time.time()
while time.time() - t0 < 60:
    n = ser.in_waiting
    if n: buf += ser.read(n)
    if b"UBOOT #" in buf or b"=> " in buf: break
    if 5 < time.time() - t0 < 6: ser.write(b"\x03")
    time.sleep(0.05)
if b"UBOOT #" not in buf and b"=> " not in buf:
    print("      NO PROMPT!", buf.decode(errors='replace')[-200:])
    sys.exit(1)
print("      OK\n")

# Upload kernel
print(f"[2] Kernel: {kern_words} words ...")
t0 = time.time()
for i in range(kern_words):
    w = struct.unpack_from('<I', kern, i*4)[0]
    ser.write(f"mw.l 0x{KERNEL_ADDR+i*4:08X} 0x{w:08X}\n".encode())
    time.sleep(0.005)
    if (i+1) % 500 == 0:
        e = time.time() - t0
        print(f"      {(i+1)*100//kern_words}%  {(i+1)/e:.0f} w/s")

# Drain buffer after kernel upload
for _ in range(50):
    n = ser.in_waiting
    if n: ser.read(n)
    else: time.sleep(0.05)
e = time.time() - t0
print(f"      Done {e:.1f}s\n")

# Upload DTB
print(f"[3] DTB: {dtb_words} words ...")
t0 = time.time()
for i in range(dtb_words):
    w = struct.unpack_from('<I', dtb, i*4)[0]
    ser.write(f"mw.l 0x{DTB_ADDR+i*4:08X} 0x{w:08X}\n".encode())
    time.sleep(0.005)
for _ in range(20):
    n = ser.in_waiting
    if n: ser.read(n)
    else: time.sleep(0.05)
e = time.time() - t0
print(f"      Done {e:.1f}s\n")

# Verify prompt before booti
print("[4] Verify prompt ...")
ser.write(b"\n")
time.sleep(1)
buf = b""
t0 = time.time()
while time.time() - t0 < 5:
    n = ser.in_waiting
    if n:
        buf += ser.read(n)
        if b"UBOOT #" in buf or b"=> " in buf: break
    time.sleep(0.05)
if b"UBOOT #" in buf or b"=> " in buf:
    print("      OK\n")
else:
    print(f"      WARN: {buf.decode(errors='replace')[-80:]}\n")

# Boot + continuous capture
print(f"[5] booti 0x{KERNEL_ADDR:08X} - 0x{DTB_ADDR:08X}")
ser.write(f"booti 0x{KERNEL_ADDR:08X} - 0x{DTB_ADDR:08X}\n".encode())

print("\n--- Output ---\n")
t0 = time.time()
last_data = 0
while time.time() - t0 < 30:
    n = ser.in_waiting
    if n:
        chunk = ser.read(n)
        sys.stdout.buffer.write(chunk)
        sys.stdout.flush()
        last_data = time.time()
    else:
        # Break if no data for 5 seconds after first data
        if last_data and time.time() - last_data > 5:
            break
        time.sleep(0.01)
time.sleep(0.5)
n = ser.in_waiting
if n: sys.stdout.buffer.write(ser.read(n))
print("\n=== Done ===")
ser.close()

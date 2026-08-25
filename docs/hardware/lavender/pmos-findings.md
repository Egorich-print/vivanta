# pmOS on Lavender — Findings from the 2026-08-21 Bring-up Session

Device: Xiaomi Redmi Note 7 (`xiaomi-lavender`, SDM660, Tianma panel + Novatek NT36672A TS).
Target: postmarketOS `edge`, kernel `linux-postmarketos-qcom-sdm660` 7.0.14, kernel
variant `tianma`, UI `phosh`, init `systemd`, user `egorich`.

Status at end of session: pmOS boots to a working system with terminal access; final
clean image (all fixes baked in) built but not yet flashed; TWRP downloaded as a
rescue path; fastboot entry from buttons proved unreliable (see §7).

Build tree: `~/ai-workstation/Projects/lavender-flasher/` (artifacts + Rust flasher).
Build VM: Lima `pmos-builder` (Alpine 3.23 aarch64, vz, 8 vCPU / 12 GiB).

---

## 1. Build environment facts

- Alpine 3.23's packaged pmbootstrap (3.10.3) is too old for pmaports `main`
  (`pmbootstrap_min_version`). Install pmbootstrap from git into a venv
  (`~/venv-pm` on the guest) and symlink into `/usr/local/bin`.
- pmbootstrap 3.11 shells out with `doas`, not `sudo`. Lima guest user is not in
  `wheel`; fix with `/etc/doas.d/doas.conf`: `permit nopass egorich as root`.
- `pmbootstrap init` (3.11) has no answer flags. Deterministic non-interactive
  init = pre-seed `~/.config/pmbootstrap_v3.cfg` directly:

  ```ini
  [pmbootstrap]
  device = xiaomi-lavender
  kernel = tianma            # panel variant = kernel subpackage
  ui = phosh
  systemd = always
  user = egorich
  hostname = lavender
  is_default_channel = False
  extra_packages = postmarketos-base-ui-networkmanager,alsa-ucm-conf-qcom-sdm660,parted,openssh

  [providers]
  postmarketos-base-ui-audio-backend = postmarketos-base-ui-audio-backend-pipewire
  postmarketos-base-ui-wifi = postmarketos-base-ui-wifi-iwd
  postmarketos-usb-moded-default-profile = postmarketos-usb-moded-default-profile-developer
  ```

- **Gotcha:** local pmaports patches are silently ignored unless `pkgrel` is
  bumped (or `pmbootstrap build --force`). pmbootstrap prefers the binary
  package from the mirror otherwise. This cost us one full debug cycle.
- `deviceinfo_flash_sparse="true"` → exported rootfs is an Android **sparse**
  image. `losetup` cannot parse it; `simg2img` (pkg `android-tools`) first.
- `avbtool` does not exist in Alpine or pmaports. A minimal disabled vbmeta is
  256-byte AVB0 header + zero padding to flash pagesize (4096):

  ```python
  struct.pack(">4sIIQQIQQQQQQQQQQQII48s80s", b"AVB0", 1, 0, 0, 0, 0,
              *([0]*10), 0, 2, 0, b"avbtool 1.3.0" + b"\0"*35, b"\0"*80)
  ```

  Bootloader accepts it: UART log shows `VERIFICATION_DISABLED bit is set →
  continue boot` on an unlocked device.
- Lima guest quirk: kernel loop-partition scan is racy — `/dev/loopXp2` may not
  appear for seconds (pmbootstrap aborts with "File did not appear"). Simply
  retrying `pmbootstrap install` gets past it.
- `pmbootstrap -y zap` deletes `cache_git/pmaports` — **local patches die with
  it**. Keep a re-apply script (`reapply-all.py` in project root) or keep
  patches as files outside the work dir.

## 2. Boot chain facts (from pmOS wiki + verified in practice)

- Bootloader appends harmful cmdline (`root=`, `skip_initramfs`, `init=/init`).
  sdm660-mainline ≥ 6.9 neutralises them by mangling the first letter
  (`_oot=`, `_kip_initramfs`, `_nit=/init`). Verified in `/proc/cmdline`.
- `fastboot erase dtbo` is correct for mainline: BL logs
  `Dtbo hdr magic mismatch → Best match DTB tags → proceeding`.
- `fastboot boot` is broken on lavender: bootloader fails to decompress the
  kernel ("Error in decompression"). Only real flashing works.
- `fastboot oem getlog` dumps the pstore console of the previous boot — the
  poor man's UART. Output is garbled but usable.
- UART test points: TP11 = TX (gpio4), TP10 = RX (gpio5), GND = shield.
- The `splash` partition (p54, 64 MiB) holds the boot logo shown by the
  bootloader until the kernel takes over the display. This device previously
  ran Ubuntu Touch, so the logo says "ubuntu touch" — it is NOT a UT install
  and says nothing about the current OS state.
- Kernel cmdline is baked into `boot.img` via the device package's
  `kernel-cmdline.conf` (installed as
  `/usr/lib/kernel-cmdline.d/50-device-xiaomi-lavender.conf`).

## 3. Root cause #1 — phantom KEY_VOLUMEUP kills every boot

`gpio-keys` on this unit reports **Volume Up permanently pressed** (stuck
hardware or DT polarity issue; `iskey KEY_VOLUMEUP` → 0 in initramfs while no
one touches the phone).

`postmarketos-initramfs` runs `check_keys()` early in `init_2nd.sh`:

| key held during first ~15 s | behaviour |
|---|---|
| Vol Down or Left Ctrl | drops to debug shell (silent) |
| Vol Up or Left Shift | `fail_halt_boot`: log dump over USB mass-storage + debug shell |
| nothing | normal boot |

Every "stuck at splash" we observed was one of these two paths, triggered by
the phantom key and/or by us holding buttons (e.g. trying to enter fastboot
*after* the kernel already started — buttons are just input events then, the
bootloader never sees them).

Fix applied in pmaports `main/postmarketos-initramfs/init_functions.sh`:
`check_keys()` reduced to `touch /tmp/debug_shell_exited`. Debug shell remains
reachable via `pmos.debug-shell` cmdline param.

System side: udev rule shipped by the device package
(`/etc/udev/rules.d/90-lavender-phantom-volup.rules`):

```
SUBSYSTEM=="input", ATTRS{name}=="gpio-keys", ENV{LIBINPUT_IGNORE_DEVICE}="1"
```

## 4. Root cause #2 — subpartitions never mounted

pmOS rootfs image (GPT with `pmOS_boot` ext2 + `pmOS_root` ext4) is flashed
into `userdata` (mmcblk1p66). Mainline kernels do **not** scan partitions
inside partitions, so `/dev/mmcblk1p66p1/p2` never exist and
`find_root_partition` (by `pmos_root_uuid=`) finds nothing.

`mount_subpartitions()` in init_functions.sh handles this via
`losetup --show -Pf --direct-io=on` — but had three defects for this device:

1. calls plain `fdisk`, which is **absent from the initramfs** →
   `part_count` always 0 → never detects the 2 inner partitions. Fixed:
   `busybox fdisk -l` (busybox has the applet; verified GPT output matches
   the counting regex).
2. iterates `/dev/disk/by-partlabel/userdata` first but the sysfs filter
   checks `/sys/class/block/$(basename …)` — the *symlink* name `userdata`
   has no sysfs entry, so the real device got probed **last** of ~70
   candidates. Fixed: `partition="$(readlink -f "$partition")"` at loop head.
3. 10 s probe window too short under early-boot load (65 fdisk forks before
   reaching p66). Fixed: `wait_seconds=30`.

With these three, root is found in < 1 s. Verified live with `set -x
mount_subpartitions` in the debug shell.

## 5. Rootfs content gaps

- **sshd is NOT installed** by default even though `pmbootstrap install`
  prints "SSH daemon is enabled". Add `openssh` to `extra_packages`
  (client-only packages were present; `/usr/sbin/sshd` was not).
- `usb-moded` default-profile **charging** = USB gadget without network
  function → no `usb0` in the booted system (initramfs networking dies at
  switch_root). The `developer` provider profile brings USB networking up in
  the running system.
- Persistent journal: `mkdir /var/log/journal` (volatile by default; we lost
  several crash logs to this).

## 6. Remote surgery toolbox (no fastboot required)

The initramfs debug shell is a full rescue environment. Entry: any boot with
`pmos.debug-shell` in cmdline, or hold Vol Down during early boot.

- USB gadget: phone = `172.16.42.1`, Mac gets `172.16.42.2` via unudhcpd
  (macOS: interface shows as enX with NCM; `ipconfig set enX DHCP` if lost).
- `pmos_continue_boot` — resumes the boot blocked by the debug shell.
- Mount the real rootfs from initramfs:

  ```sh
  losetup -f --show -P /dev/mmcblk1p66     # → /dev/loopN with loopNp1/p2
  mount /dev/loopNp2 /sysroot
  chroot /sysroot /bin/sh                  # full Alpine userland + apk
  ```

- File transfer Mac → phone: `python3 -m http.server 9999 --bind 172.16.42.2`
  on the Mac, `wget -O /tmp/x http://172.16.42.2:9999/x` in initramfs
  (busybox wget/nc/dd/sha256sum all present).
- **Live-partition flash** (kernel+initramfs run from RAM, safe):
  `dd if=/tmp/boot.img of=/dev/mmcblk1p60 bs=1048576; sync` then verify with
  `dd if=/dev/mmcblk1p60 bs=<size> count=1 | sha256sum`.
- Install packages into the mounted rootfs: serve a dir with the signed
  `APKINDEX.tar.gz` + `.apk` files under `repo/aarch64/` (apk appends the
  arch dir), then
  `chroot /sysroot /sbin/apk --repositories-file /tmp/repo.list add openssh`.
- Post-mortem: `mount -t pstore pstore /sys/fs/pstore` in the debug shell →
  `console-ramoops-0` holds the previous boot's kernel console.
- `pmos_logdump` (banner command) exports initramfs logs as USB mass-storage.

## 7. Fastboot / reboot notes

- From a running system, plain `reboot bootloader` / `reboot fastboot` does
  **not** work (busybox/systemd drop the reboot argument). Correct form on
  systemd: `systemctl reboot --reboot-argument=bootloader` (untested —
  session ended before verification).
- Button combos only work in the bootloader, i.e. from a *truly powered-off*
  state: Power held ~20 s until screen dies, wait 3 s, then
  Vol− + Power → fastboot, Vol+ + Power → recovery.
  Catching this window on this unit proved unreliable in practice.
- Rescue path prepared but not yet used: TWRP
  `twrp-3.7.1_12-1-lavender-20240825-2142.img` (SourceForge `lavender-roms`,
  4.19 branch) → `fastboot flash recovery …`. TWRP boots its own downstream
  kernel, independent of the pmOS chain; also useful to test whether the
  phantom Vol Up auto-triggers recovery on boot.

## 8. Wi-Fi / audio state

- Wi-Fi fix (WCN3990): `/etc/modprobe.d/ath10k.conf` =
  `options ath10k_core skip_otp=y cryptmode=1`, shipped by the device package;
  NM powersave off via `/etc/NetworkManager/conf.d/70-wifi-powersave.conf`
  (`wifi.powersave = 2`). Known upstream issue remains: modem crash on Wi-Fi
  disconnect (remoteproc fatal) — avoid disconnects until fixed upstream.
- Audio: `alsa-ucm-conf-qcom-sdm660` provides
  `ucm2/conf.d/sdm660-internal/Xiaomi Redmi Note 7.conf`; PipeWire +
  WirePlumber route via UCM (`postmarketos-base-ui-audio-backend-pipewire`
  provider). Not yet verified on hardware.

## 9. Artifacts (build_artifacts/)

| file | what |
|---|---|
| `boot-final.img` | final boot: clean cmdline (`msm.prefer_mdp5=false`), fixed initramfs, sshd in rootfs |
| `xiaomi-lavender.img` | final sparse rootfs (openssh, developer usb-moded, udev rule) |
| `vbmeta.img` | disabled AVB (flags=2), 4 KiB |
| `twrp-3.7.1_12-lavender.img` | rescue recovery (not flashed yet) |
| `boot-v*.img`, `boot.img`, `boot-debug.img` | debug iterations, kept for archaeology |

Flash order when fastboot is available:
`flash recovery twrp → flash vbmeta → erase dtbo → flash boot → flash userdata → reboot`.

## 10. Open items

1. Flash final images (fastboot, or dd-from-running-system via SSH over Wi-Fi).
2. Verify `systemctl reboot --reboot-argument=bootloader` actually lands in
   fastboot.
3. Confirm Phosh on Tianma panel; SSH over USB (developer profile); Wi-Fi
   connect/disconnect behaviour; audio endpoints via UCM.
4. Investigate phantom Volume Up: hardware (disassemble, check the button) vs
   DT (gpio-keys polarity in sdm660-xiaomi-lavender-tianma dts).
5. Upstream the initramfs fixes (busybox fdisk, readlink, window) — they are
   generic, not lavender-specific.

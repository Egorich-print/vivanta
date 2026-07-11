# Lavender Boot Notes

## Open questions

- Is fastboot available with unlocked bootloader?
- Can we use `fastboot boot boot.img` or must we flash?
- Where does the DTB live in the boot image?
- What is the entry point address?

## Commands to try (once device available)

```bash
# Enter fastboot
adb reboot bootloader

# Test boot (without flashing)
fastoot boot theseus-boot.img

# UART
# Find UART pins on the motherboard
```

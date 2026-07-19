fn main() {
    println!("cargo:rustc-link-arg=-Ttarget-qemu-armv7a/linker.ld");
}

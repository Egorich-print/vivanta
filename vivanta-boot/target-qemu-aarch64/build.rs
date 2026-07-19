fn main() {
    println!("cargo:rustc-link-arg=-Ttarget-qemu-aarch64/linker.ld");
}

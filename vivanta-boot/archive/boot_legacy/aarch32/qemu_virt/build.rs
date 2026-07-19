fn main() {
    println!("cargo:rustc-link-arg=-Tboot/aarch32/qemu_virt/linker.ld");
}

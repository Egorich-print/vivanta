fn main() {
    println!("cargo:rustc-link-arg=-Ttarget-mmu-test/linker.ld");
}

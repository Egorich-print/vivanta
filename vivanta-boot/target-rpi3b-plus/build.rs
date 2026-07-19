fn main() {
    println!("cargo:rustc-link-arg=-Ttarget-rpi3b-plus/linker.ld");
}

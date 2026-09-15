fn main() {
    // Link the naked EL0 image at the fixed user VA (0x01000000) via the
    // package-local script. Absolute path: robust to cargo's CWD in both
    // standalone ([workspace]) and workspace-member builds.
    println!(
        "cargo:rustc-link-arg=-T{}/user-init.ld",
        env!("CARGO_MANIFEST_DIR")
    );
    println!("cargo:rerun-if-changed=user-init.ld");
}

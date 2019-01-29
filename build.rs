use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Only re-run the build script when memory.x is changed,
    // instead of when any part of the source code changes.
    println!("cargo:rerun-if-changed=memory.x");

    cc::Build::new()
        .file("src/asm/unpack_1bpp.S")
        .file("src/asm/unpack_1bpp_overlay.S")
        .file("src/asm/copy_words.S")
        .compile("libunrusted.a");

    cc::Build::new()
        .file("src/bin/xor_pattern/pattern.S")
        .compile("libxor_pattern.a");
}

//! Builds the vendored ares SFC core (ISC; see vendor/ares/LICENSE.txt) as a
//! static library, plus the project-authored headless shim.
//!
//! Only the Super Famicom core is compiled (via the project-authored unity
//! source), with nall (main/nall/sljitAllocator), libco (per-arch), and the
//! vendored sljit JIT dependency. No GUI, no debugging server.

use std::env;
use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest.join("../../vendor/ares");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Embed the system-pak data (SPC IPL + board database) for the shim.
    let boards = std::fs::read(vendor.join("ares/System/Super Famicom/boards.bml")).unwrap();
    let ipl = std::fs::read(vendor.join("ares/System/Super Famicom/ipl.rom")).unwrap();
    let mut hdr = String::new();
    hdr.push_str("#pragma once\n#include <cstdint>\n#include <cstddef>\n");
    hdr.push_str("static const uint8_t gBoardsBml[] = {");
    for b in &boards {
        let _ = write!(hdr, "0x{b:02x},");
    }
    hdr.push_str("};\nstatic const size_t gBoardsBmlSize = sizeof(gBoardsBml);\n");
    hdr.push_str("static const uint8_t gIplRom[] = {");
    for b in &ipl {
        let _ = write!(hdr, "0x{b:02x},");
    }
    hdr.push_str("};\nstatic const size_t gIplRomSize = sizeof(gIplRom);\n");
    std::fs::write(out.join("ares-embedded-data.hpp"), hdr).unwrap();

    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .flag("-fno-char8_t")
        .flag("-fwrapv")
        .flag_if_supported("-fno-strict-aliasing")
        .flag_if_supported("-Wno-unused")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-parentheses")
        .flag_if_supported("-Wno-switch")
        .flag_if_supported("-Wno-empty-body")
        .flag_if_supported("-Wno-comma")
        .flag_if_supported("-Wno-unknown-warning-option")
        .define("SLJIT_HAVE_CONFIG_PRE", "1")
        .define("SLJIT_HAVE_CONFIG_POST", "1")
        .include(&vendor)
        .include(vendor.join("nall"))
        .include(vendor.join("ares"))
        .include(vendor.join("thirdparty"))
        .include(&out)
        .opt_level(2)
        .warnings(false);

    let libco_arch = match arch.as_str() {
        "x86_64" => "amd64.c",
        "aarch64" => "aarch64.c",
        "x86" => "x86.c",
        other => panic!("unsupported libco arch: {other}"),
    };
    build
        .file(vendor.join("ares-unity.cpp"))
        .file(vendor.join("nall/nall/main.cpp"))
        .file(vendor.join("nall/nall/nall.cpp"))
        .file(vendor.join("nall/nall/sljitAllocator.cpp"))
        .file(vendor.join("libco/libco.c"))
        .file(vendor.join(format!("libco/{libco_arch}")))
        .file(vendor.join("thirdparty/sljit/sljit_src/sljitLir.c"))
        .file(vendor.join("shims.cpp"));
    build.compile("ares_oracle");

    println!("cargo:rerun-if-changed={}", vendor.display());
}

//! Compile only the vendored audio core and the private streaming shim.
use std::path::Path;

fn main() {
    let vendor = Path::new("../../../vendor/lakesnes");
    let mut build = cc::Build::new();
    build
        .include(vendor)
        .warnings(false)
        .flag_if_supported("-std=c99");
    for name in ["apu.c", "spc.c", "dsp.c", "statehandler.c"] {
        let path = vendor.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        build.file(path);
    }
    for entry in std::fs::read_dir(vendor).expect("vendored LakeSnes directory") {
        let path = entry.expect("vendor entry").path();
        if path.extension().is_some_and(|ext| ext == "h") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    println!("cargo:rerun-if-changed=shim.c");
    build.file("shim.c").compile("spc_player");
}

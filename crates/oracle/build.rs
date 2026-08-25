//! Builds the vendored `LakeSnes` core (MIT, `angelo_wf`/`elzo_d` and
//! contributors; see vendor/lakesnes/LICENSE.txt) as a static library.
//!
//! Only the headless core is compiled: no SDL, no windowing, no audio
//! devices. Output goes to caller-provided buffers.

fn main() {
    let files = [
        "apu.c",
        "cart.c",
        "cpu.c",
        "dma.c",
        "dsp.c",
        "input.c",
        "ppu.c",
        "snes.c",
        "snes_other.c",
        "spc.c",
        "statehandler.c",
        "shims.c",
    ];
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = root.join("../../vendor/lakesnes");
    let mut build = cc::Build::new();
    build
        .files(files.iter().map(|f| vendor.join(f)))
        .include(&vendor)
        .warnings(false) // upstream code style; we do not maintain warnings here
        .flag_if_supported("-std=c99");
    build.compile("lakesnes");
    println!("cargo:rerun-if-changed={}", vendor.display());
}

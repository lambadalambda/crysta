//! Native evidence producer, deliberately separate from the host-free library.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: web-spike ROM | --replay")?;
    if path == "--replay" {
        println!("{}", web_spike::replay());
    } else {
        // Blocking I/O and the clock exist only in this native harness.
        let input = std::fs::read(path)?;
        let start = std::time::Instant::now();
        let report = web_spike::probe(&input)?;
        eprintln!(
            "native probe (validation + decode + replay): {:?}",
            start.elapsed()
        );
        println!("{report}");
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}

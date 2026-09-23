//! Authenticated source packets for the bounded class-0 cadence witness.
//! JSON goes to the verifier over a pipe; no extracted assets are retained.
use assets::compression;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    assert_eq!(args.len(), 2, "crysta-cadence-decode ROM");
    let bytes = std::fs::read(&args[1])?;
    let rom = rom::Rom::load(&bytes)?;
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let image = rom.image();

    // Map-D spawn, descriptor and class-0 pose/movement pairs.
    assert_eq!(
        &image[0x38cb4..0x38cbe],
        &[1, 4, 0x2a, 0, 0x37, 0xa8, 0x88, 0xeb, 0xed, 0x83]
    );
    assert_eq!(&image[0x3edeb..0x3edef], &[0x22, 0x10, 0xd8, 0x20]);
    assert_eq!(
        &image[0x8f6d..0x8f75],
        &[3, 0x68, 4, 0x69, 5, 0x60, 5, 0x60]
    );
    for site in [0x18817d, 0x188272] {
        assert_eq!(&image[site..site + 7], &[1, 1, 2, 0, 0x37, 0xf0, 9]);
        let packed = u32::from_le_bytes([image[site + 4], image[site + 5], image[site + 6], 0]);
        assert_eq!(
            ((0x98 + (packed >> 15)) << 16) | (packed & 0x7fff) | 0x8000,
            0xabf037
        );
    }
    let decode = |start: usize, end: usize, size: usize| {
        let packet = compression::decode(&image[start..(start | 0xffff) + 1], size).unwrap();
        assert_eq!(start + packet.consumed, end);
        assert_eq!(packet.data.len(), size);
        packet.data
    };
    let common = decode(0x2bf037, 0x2bf8f7, 0x1a0c);
    let body = decode(0x181022, 0x18118a, 0x28b);

    // Town class-2 walkers: $83:8A19 carries descriptor $83:ED37 (mode $22,
    // class = mode & $0F = 2, base $6000); $8A23 and the sampled $8A2D reuse it.
    assert_eq!(
        &image[0x38a19..0x38a37],
        &[
            1, 0x1c, 0x1a, 0, 0xad, 0x80, 0x88, 0x37, 0xed, 0x83, //
            1, 0x27, 0x1d, 0, 0xad, 0x80, 0x88, 0, 0, 0, //
            1, 0x23, 0x1a, 0, 0x03, 0x81, 0x88, 0, 0, 0,
        ]
    );
    assert_eq!(&image[0x3ed37..0x3ed3b], &[0x90, 0x78, 0xd4, 0x22]);
    let town_body = decode(0x147890, 0x147bf2, 0x71d);
    println!("{}", json!({"common": common, "body": body, "town_body": town_body}));
    Ok(())
}

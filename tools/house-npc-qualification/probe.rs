//! Experimental source qualification: real menu prefix, no state restoration.
use oracle::{Button, Session};
#[path = "../new-game-qualification/bootstrap.rs"]
mod bootstrap;
const ROUTE: &[(Button, u32, u32)] = &[(Button::Right, 6800, 6862), (Button::Down, 6900, 6967)];
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(a.len(), 3, "probe ROM local/OUT");
    let out = std::path::Path::new(&a[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    std::fs::create_dir(out).unwrap();
    let rom = rom::Rom::load(&std::fs::read(&a[1]).unwrap()).unwrap();
    assert_eq!(rom.revision(), rom::Revision::Japan);
    let mut s = Session::new(&rom).unwrap();
    for label in 0..7300 {
        for b in [
            Button::Start,
            Button::A,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
        ] {
            s.set_button(
                b,
                bootstrap::INPUTS
                    .iter()
                    .chain(ROUTE)
                    .any(|&(v, start, end)| v == b && (start..end).contains(&label)),
            );
        }
        s.run_frame();
        let f = label + 1;
        if [6800, 7050, 7100, 7200, 7300].contains(&f) {
            std::fs::write(out.join(format!("f{f}.wram")), s.wram_image()).unwrap();
            for (name, words) in [("vram", s.vram()), ("cgram", s.cgram())] {
                std::fs::write(
                    out.join(format!("f{f}.{name}")),
                    words
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            }
            std::fs::write(out.join(format!("f{f}.pixels")), s.pixels()).unwrap();
            let hw = s.sprite_state();
            std::fs::write(out.join(format!("f{f}.oam")), hw.oam).unwrap();
            std::fs::write(out.join(format!("f{f}.obj")), [hw.obsel, hw.first_sprite]).unwrap();
        }
    }
    // ares teardown is not needed in this one-session capture process.
    std::process::exit(0);
}

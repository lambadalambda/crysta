//! The world map's Mode 7 view against native frames
//! (`docs/world-map-mode7.md`): `local/mode7/{jp,eu}/<frame>/native.rgb`,
//! with the player position each was drawn at.
#[path = "support/native.rs"]
mod native;
use assets::graphics::Bgr555;
use assets::maps::visual::mode7::Mode7View;
use assets::maps::visual::world::WorldMap;
use crysta_app::frame::{Canvas, CLASSIC_WIDTH};
use crysta_app::mode7::{self, Mode7};
use native::{local, unlike_where};
use rom::Rom;

#[test]
fn the_view_matches_native_frames_on_both_roms() {
    for (rom, revision, offset, frames) in [
        (
            "Tenchi Souzou (Japan).sfc",
            "jp",
            (0, 9),
            [
                ("arrival-59762", (536, 544)),
                ("walk-down-59768", (536, 552)),
                ("walk-left-59778", (524, 560)),
            ],
        ),
        (
            "Terranigma (E) [!].smc",
            "eu",
            (-2, 29),
            [
                ("arrival-73938", (536, 544)),
                ("walk-down-73942", (536, 548)),
                ("walk-down-73950", (536, 564)),
            ],
        ),
    ] {
        let Some(bytes) = local(rom) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let mode7 = Mode7 {
            map: WorldMap::from_rom(rom.image(), 0x0003).unwrap(),
            view: Mode7View::from_rom(rom.image()).unwrap(),
        };
        for (frame, player) in frames {
            let native = local(&format!("mode7/{revision}/{frame}/native.rgb"))
                .expect("with the ROM, its captures in `local/mode7`");
            // The palette as the frame had it: the lava and ice cycle
            // (`$87:98BF`), which this test leaves to the snapshot.
            let cgram = local(&format!("mode7/{revision}/{frame}/prev.cgram")).unwrap();
            let mut mode7 = Mode7 {
                map: mode7.map.clone(),
                view: mode7.view.clone(),
            };
            mode7.map.set_palette(std::array::from_fn(|i| {
                Bgr555::new(u16::from_le_bytes([cgram[2 * i], cgram[2 * i + 1]]))
            }));
            let mut canvas = Canvas::new(CLASSIC_WIDTH);
            mode7::draw(&mut canvas, &mode7, player);
            // Left out: the sprites (Ark, the horizon's band), by their
            // boxes in the frame's OAM: 8x8, or 16x16 when large (OBSEL 2).
            let oam = local(&format!("mode7/{revision}/{frame}/prev.oam")).unwrap();
            let boxes: Vec<(usize, usize, usize)> = (0..128)
                .filter_map(|n| {
                    let high = oam[512 + n / 4] >> (2 * (n % 4));
                    let size = if high & 2 != 0 { 16 } else { 8 };
                    let x = usize::from(oam[4 * n]);
                    let y = usize::from(oam[4 * n + 1]) + 1;
                    (high & 1 == 0 && y < 224).then_some((x, y, size))
                })
                .collect();
            let keep = |x: usize, y: usize| {
                !boxes.iter().any(|&(bx, by, size)| {
                    (bx..bx + size).contains(&x) && (by..by + size).contains(&y)
                })
            };
            let unlike = unlike_where(&canvas, &native, offset, keep);
            assert_eq!(unlike, 0, "{revision} {frame}");
        }
    }
}

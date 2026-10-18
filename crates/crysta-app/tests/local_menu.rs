//! Yomi's box drawn from the ROM against native frames (`docs/menu.md`):
//! `local/menu/*-hall.rgb`, 256×240 RGB captures of the hall 80 frames
//! after Select, Yomi on the item door.
#[path = "support/native.rs"]
mod native;
use crysta_app::frame::{Canvas, CLASSIC_WIDTH};
use crysta_app::session::{Buttons, Session};
use native::{local, unlike_where};
use rom::Rom;

/// The hall with Yomi, as the session draws it 80 frames after Select.
fn hall(rom: &Rom) -> Canvas {
    let image: &'static [u8] = Box::leak(rom.image().to_vec().into_boxed_slice());
    let mut events = crysta_runtime::world::fresh_game_flags();
    events[0xFE / 8] |= 1 << (0xFE % 8);
    let mut session = Session::new(image);
    session.world =
        crysta_runtime::world::World::enter_with_events(image, 0x0102, 520, 460, events).unwrap();
    session.advance_with(
        None,
        Buttons {
            select: true,
            ..Buttons::default()
        },
    );
    for _ in 0..79 {
        session.advance(None, false, false);
    }
    let mut canvas = Canvas::new(CLASSIC_WIDTH);
    session.compose(rom, &mut canvas);
    canvas
}

#[test]
fn the_hall_and_yomi_match_native_frames_on_both_roms() {
    // Where the captures frame the view: the PAL one 20 lines lower and 2
    // pixels to the left.
    for (rom, prefix, offset) in [
        ("Tenchi Souzou (Japan).sfc", "jp", (0, 8)),
        ("Terranigma (E) [!].smc", "eu", (-2, 28)),
    ] {
        let Some(bytes) = local(rom) else {
            continue;
        };
        let rom = Rom::load(&bytes).unwrap();
        let native = local(&format!("menu/{prefix}-hall.rgb"))
            .expect("with the ROM, its captures in `local/menu`");
        let canvas = hall(&rom);
        // The wall lamps' tile animation is not drawn yet (cells (2,2),
        // (14,2), (1,4), (15,4)).
        let lamp = |x: usize, y: usize| {
            [(2, 2), (14, 2), (1, 4), (15, 4)].contains(&(x / 16, (y + 8) / 16))
        };
        // The item's icon (OBJ, (160,8)) is not drawn yet either.
        let item = |x: usize, y: usize| (160..176).contains(&x) && (8..24).contains(&y);
        let keep = |x: usize, y: usize| !lamp(x, y) && !item(x, y);
        let unlike = unlike_where(&canvas, &native, offset, keep);
        assert_eq!(unlike, 0, "{prefix}");
    }
}

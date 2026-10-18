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

#[test]
fn the_weapon_rooms_pedestals_match_a_native_frame() {
    // `local/menu/jp-weapon.rgb`: `$81` in slot 4 (`$7F:8048`), the
    // weapon door taken 30 frames after a Left, 200 frames on: the icons,
    // the empty pedestals' black, the room's colours, Yomi spawned again.
    // The rooms' labels, the lamps and the item icon are not drawn yet.
    let Some(bytes) = local("Tenchi Souzou (Japan).sfc") else {
        return;
    };
    let rom = Rom::load(&bytes).unwrap();
    let native = local("menu/jp-weapon.rgb").expect("with the ROM, its capture in `local/menu`");
    let image: &'static [u8] = Box::leak(rom.image().to_vec().into_boxed_slice());
    let mut events = crysta_runtime::world::fresh_game_flags();
    events[0xFE / 8] |= 1 << (0xFE % 8);
    let mut session = Session::new(image);
    session.world =
        crysta_runtime::world::World::enter_with_events(image, 0x0102, 520, 460, events).unwrap();
    session.world.give_item(0x81);
    let select = Buttons {
        select: true,
        ..Buttons::default()
    };
    session.advance_with(None, select);
    for _ in 0..79 {
        session.advance(None, false, false);
    }
    session.advance(Some(room_core::Direction::Left), false, false);
    for _ in 0..30 {
        session.advance(None, false, false);
    }
    session.advance(None, true, false);
    for _ in 0..200 {
        session.advance(None, false, false);
    }
    let mut canvas = Canvas::new(CLASSIC_WIDTH);
    session.compose(&rom, &mut canvas);
    let lamp = |x: usize, y: usize| [(2, 2), (14, 2), (1, 4), (15, 4)].contains(&(x / 16, (y + 8) / 16));
    let item = |x: usize, y: usize| (160..176).contains(&x) && (8..24).contains(&y);
    let keep = |x: usize, y: usize| y < 176 && !lamp(x, y) && !item(x, y);
    assert_eq!(unlike_where(&canvas, &native, (0, 8), keep), 0);
}

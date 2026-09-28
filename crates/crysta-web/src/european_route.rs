//! Fresh European browser-host input routes from Elle through the blue door.
//! Walking legs, Box tour and frozen-town detour mirror `runtime/tests/local_european.rs`.
//! Set `CRYSTA_WEB_EU_TRACE=/tmp/eu.trace` for the door route, or
//! `CRYSTA_WEB_EU_DETOUR_TRACE=/tmp/eu-detour.trace` for the full detour.
use super::{buttons, Game};
use room_core::Direction;
use std::fmt::Write as _;
use std::path::PathBuf;

use buttons::{CONFIRM as X, DOWN as D, LEFT as L, RIGHT as R, UP as U};

// Run-length count, held bits, extra action presses.
type Trace = Vec<(usize, u32, u32)>;

struct Route {
    game: Game,
    trace: Option<(PathBuf, Trace)>,
}

impl Route {
    fn new(bytes: &[u8], trace_env: &str) -> Self {
        Self {
            game: Game::new(bytes).unwrap(),
            trace: std::env::var_os(trace_env).map(|path| (path.into(), Vec::new())),
        }
    }

    fn frame(&mut self, held: u32) {
        if let Some((_, trace)) = &mut self.trace {
            if let Some((count, previous, presses)) = trace.last_mut() {
                if (*previous, *presses) == (held, 0) {
                    *count += 1;
                } else {
                    trace.push((1, held, 0));
                }
            } else {
                trace.push((1, held, 0));
            }
        }
        self.game.frame(held);
        assert_eq!(self.game.fault(), None, "host frame stopped");
    }

    fn frames(&mut self, count: usize, held: u32) {
        for _ in 0..count {
            self.frame(held);
        }
    }

    fn legs(&mut self, legs: &[(usize, u32)]) {
        for &(count, held) in legs {
            self.frames(count, held);
        }
    }

    fn until(&mut self, limit: usize, held: u32, reached: impl Fn(&Self) -> bool) {
        for _ in 0..limit {
            if reached(self) {
                return;
            }
            self.frame(held);
        }
        assert!(
            reached(self),
            "failed to walk {held:#x} in map={:#x} at {:?}",
            self.world().map(),
            self.world().position()
        );
    }

    fn press_x(&mut self) {
        self.frame(X);
        self.frame(0); // Separate press for the next lift, throw or page.
    }

    fn world(&self) -> &crysta_runtime::world::World<'static> {
        &self.game.session.world
    }

    fn finish_scene(&mut self, limit: usize) -> bool {
        let mut chose = false;
        let mut quiet = 0;
        for frame in 0..limit {
            let world = self.world();
            let choosing = world.dialogue().and_then(|view| view.cursor).is_some();
            let confirm = world.dialogue().is_some() && !world.typing() && frame % 2 == 0;
            chose |= choosing && confirm;
            self.frame(if confirm { X } else { 0 });
            let world = self.world();
            if !world.in_scene() && world.dialogue().is_none() && !world.pad_locked() {
                quiet += 1;
                if quiet == 30 {
                    return chose;
                }
            } else {
                quiet = 0;
            }
        }
        panic!(
            "scene stuck: map={:#x} pos={:?} typing={} frozen={:x?}",
            self.world().map(),
            self.world().position(),
            self.world().typing(),
            self.world().frozen_scripts()
        );
    }

    fn save_trace(&self) {
        if let Some((path, trace)) = &self.trace {
            let mut text = String::from("# count held pressed (web button bits, hex)\n");
            for &(count, held, pressed) in trace {
                writeln!(text, "{count} {held:#x} {pressed:#x}").unwrap();
            }
            std::fs::write(path, text).unwrap();
        }
    }
}

fn flag(route: &Route, id: usize) -> bool {
    route.world().events()[id / 8] & (1 << (id % 8)) != 0
}

// Run the refusal/retry and separate-pot route once per fresh Game; hand back
// the live host at $20 so later tests can continue without seeding state.
#[allow(clippy::too_many_lines)] // One input-only journey, with checkpoints at each transition.
fn opened_blue_door(bytes: &[u8], trace_env: &str) -> Route {
    let mut route = Route::new(bytes, trace_env);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x0f, (304, 112))
    );
    for _ in 0..400 {
        if route.world().dialogue().is_some() {
            break;
        }
        route.frame(0);
    }
    assert!(
        route.world().dialogue().is_some(),
        "Elle speaks in the bedroom"
    );
    route.finish_scene(6000);
    assert!(flag(&route, 0x20));
    assert_eq!(route.world().items(), [0x7a, 0xa0]);

    route.legs(&[(62, R), (38, 0), (67, D), (83, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x10, (392, 353))
    );
    for (leg, &(frames, held)) in [
        (42, D),
        (78, L),
        (90, 0),
        (12, L),
        (100, 0),
        (55, L),
        (70, U),
        (100, 0),
        (1, U),
        (100, 0),
        (50, U),
        (100, 0),
        (180, U),
    ]
    .iter()
    .enumerate()
    {
        if leg == 8 {
            route.frame(U | X); // Open the house door with a fresh X edge.
        } else {
            route.frames(frames, held);
        }
        if leg == 4 {
            assert_eq!(route.world().map(), 0x0c, "crossed into C");
        }
    }
    assert_eq!(route.world().map(), 0x0b);
    route.finish_scene(4000);
    route.legs(&[(180, U), (100, 0)]);
    assert_eq!(
        route.world().position(),
        (120, 128),
        "Elder approached on foot"
    );
    assert!(!flag(&route, 0x26));
    route.press_x();
    for frame in 0..4000 {
        if route
            .world()
            .dialogue()
            .and_then(|page| page.cursor)
            .is_some()
        {
            break;
        }
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        route.frame(if confirm { X } else { 0 });
    }
    assert!(
        route
            .world()
            .dialogue()
            .and_then(|page| page.cursor)
            .is_some(),
        "Elder's choice"
    );
    assert!(flag(&route, 0x26));
    assert!(route.finish_scene(4000), "Elder's answer selected");

    route.legs(&[
        (60, D),
        (100, 0),
        (10, L),
        (82, D),
        (100, 0),
        (45, D),
        (140, 0),
        (22, D),
        (25, 0),
        (15, 0),
        (140, 0),
    ]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x0a, (504, 769))
    );
    for (leg, &(frames, held)) in [
        (32, D),
        (60, 0),
        (24, R),
        (90, 0),
        (120, L),
        (60, 0),
        (300, U),
        (60, 0),
        (132, R),
        (60, 0),
        (65, U),
        (60, 0),
        (56, L),
        (60, 0),
        (25, U),
        (100, 0),
        (1, U),
        (100, 0),
        (40, U),
        (180, 0),
        (43, U),
        (60, 0),
        (22, L),
        (60, 0),
        (20, U),
        (60, 0),
    ]
    .iter()
    .enumerate()
    {
        if leg == 16 {
            assert_eq!(route.world().facing(), Direction::Up);
            route.press_x();
        } else {
            route.frames(frames, held);
        }
    }
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x13, (360, 144))
    );
    assert!(!flag(&route, 0x28));
    route.press_x();
    assert!(route.finish_scene(4000), "weaver's answer selected");
    assert!(flag(&route, 0x28));

    for (leg, &(frames, held)) in [
        (22, D),
        (40, 0),
        (22, R),
        (40, 0),
        (55, D),
        (160, 0),
        (10, D),
        (40, 0),
        (55, R),
        (40, 0),
        (55, D),
        (40, 0),
        (132, L),
        (40, 0),
        (280, D),
        (80, 0),
        (101, R),
        (40, 0),
        (38, U),
        (60, 0),
        (1, 0),
        (100, 0),
        (40, U),
        (160, 0),
        (100, U),
        (160, 0),
    ]
    .iter()
    .enumerate()
    {
        if leg == 20 {
            route.press_x(); // Open the house door.
        } else {
            route.frames(frames, held);
        }
        if leg == 13 {
            // The portable town sidewalk stops seven pixels before the native lane.
            assert_eq!(
                (route.world().map(), route.world().position().1),
                (0x0a, 400)
            );
            for _ in 0..16 {
                if route.world().position().0 <= 356 {
                    break;
                }
                route.frame(L);
            }
            route.frames(40, 0);
            assert!(route.world().position().0 <= 356);
        }
        if leg == 23 {
            assert_eq!(route.world().map(), 0x0d, "returned through D");
        }
    }
    assert_eq!(route.world().map(), 0x0c);
    assert!(
        route.world().dialogue().is_some(),
        "friend's request is on screen"
    );
    assert!(flag(&route, 0x27));
    assert!(!flag(&route, 0x2e) && !flag(&route, 0x2f));

    // Reject with a Down edge, then accept the retry without moving its cursor.
    let (mut answers, mut moved_cursor) = (0, false);
    let (mut saw_locked_retry, mut saw_unlock) = (false, false);
    let mut later_pages = Vec::new();
    for frame in 0..9000 {
        if saw_locked_retry {
            if let Some(page) = route.world().dialogue() {
                let source = page.page.boundary_source();
                if !later_pages.contains(&source) {
                    later_pages.push(source);
                }
            }
        }
        let cursor = route.world().dialogue().and_then(|page| page.cursor);
        let ready = route.world().dialogue().is_some() && !route.world().typing();
        let held = if cursor.is_some() && answers == 0 && !moved_cursor {
            moved_cursor = true;
            D
        } else if cursor.is_some() && answers < 2 {
            answers += 1;
            X
        } else if (ready && frame % 2 == 0) || (flag(&route, 0x2f) && frame % 240 == 0) {
            X
        } else {
            0
        };
        route.frame(held);
        if held == D {
            assert_ne!(
                route.world().dialogue().and_then(|page| page.cursor),
                cursor,
                "Down moves the friend's choice cursor"
            );
        }
        if !saw_locked_retry && [0x2f, 0x3f, 0x42].iter().all(|&id| flag(&route, id)) {
            assert!(!flag(&route, 0x2e) && !flag(&route, 0x0b));
            assert!(
                route.world().pad_locked(),
                "retry owns the pad before player script"
            );
            saw_locked_retry = true;
        }
        if flag(&route, 0x0b) && !saw_unlock {
            assert_eq!(
                later_pages,
                [0x88_af55, 0x88_af85],
                "player pages precede $0B"
            );
            saw_unlock = true;
        }
        if answers == 2 && saw_unlock && !route.world().pad_locked() {
            break;
        }
    }
    assert!(saw_locked_retry && saw_unlock && moved_cursor);
    assert_eq!(answers, 2, "refusal then retry acceptance");
    assert!(flag(&route, 0x2f) && flag(&route, 0x3f) && flag(&route, 0x42));
    assert!(!flag(&route, 0x2e), "not the direct acceptance branch");
    assert_eq!(later_pages, [0x88_af55, 0x88_af85, 0x88_afbd]);
    assert!(flag(&route, 0x0b) && !route.world().pad_locked());
    route.frames(300, 0);
    assert!(!route.world().in_scene() && route.world().dialogue().is_none());
    assert!(route.world().frozen_scripts().is_empty());
    assert_eq!(route.world().player_script_frozen_at(), None);

    // C: lift, miss once, then land two separate throws at the blue door.
    route.legs(&[(11, R), (60, 0), (67, U), (80, 0), (24, L), (60, 0)]);
    assert_eq!(route.world().position(), (104, 352));
    route.press_x(); // First lift.
    route.frames(120, 0);
    assert_eq!(route.world().pot().map(|pot| pot.tile), Some(0xfa));
    assert!(route.world().patched_cells().contains(&(5, 21, 0xf8)));
    // Unlike direct acceptance, retry leaves this lane open: 55 Right frames
    // would carry Ark to x=185. Turn back on foot to the same x=136 miss lane.
    route.legs(&[
        (22, D),
        (60, 0),
        (23, R),
        (60, 0),
        (2, L),
        (60, 0),
        (1, U),
        (40, 0),
    ]);
    assert_eq!(
        route.world().position(),
        (136, 368),
        "align with the miss lane after retry"
    );
    route.press_x(); // First throw: a miss.
    route.frames(180, 0);
    assert_eq!(route.world().position(), (136, 368));
    assert!(route.world().pot().is_none());
    assert!(!route.world().patched_cells().contains(&(11, 21, 0x181)));
    assert!(!flag(&route, 0x292));

    route.legs(&[
        (55, L),
        (60, 0),
        (14, U),
        (60, 0),
        (16, L),
        (60, 0),
        (20, U),
        (60, 0),
        (1, R),
        (30, 0),
    ]);
    assert_eq!(route.world().position(), (40, 352));
    assert_eq!(route.world().facing(), Direction::Right);
    route.press_x(); // Second lift.
    route.frames(120, 0);
    assert_eq!(route.world().pot().map(|pot| pot.tile), Some(0xfa));
    assert!(route.world().patched_cells().contains(&(3, 21, 0xf8)));
    route.legs(&[
        (45, D),
        (60, 0),
        (22, D),
        (60, 0),
        (66, R),
        (60, 0),
        (40, U),
        (60, 0),
        (33, R),
        (60, 0),
        (20, U),
        (80, 0),
    ]);
    assert_eq!(route.world().position(), (184, 368));
    route.press_x(); // Second throw: first hit.
    route.frames(180, 0);
    assert!(route.world().patched_cells().contains(&(11, 21, 0x181)));
    assert!(route.world().pot().is_none());
    assert!(!flag(&route, 0x292), "one hit does not open the door");
    route.finish_scene(4000);

    route.legs(&[
        (16, D),
        (60, 0),
        (33, L),
        (60, 0),
        (28, U),
        (60, 0),
        (33, L),
        (60, 0),
    ]);
    assert_eq!(route.world().position(), (88, 352));
    assert_eq!(route.world().facing(), Direction::Left);
    route.press_x(); // Third lift.
    route.frames(120, 0);
    assert_eq!(route.world().pot().map(|pot| pot.tile), Some(0xfb));
    assert!(route.world().patched_cells().contains(&(4, 21, 0xf8)));
    route.legs(&[
        (33, R),
        (60, 0),
        (28, D),
        (60, 0),
        (33, R),
        (60, 0),
        (20, U),
        (80, 0),
    ]);
    assert_eq!(route.world().position(), (184, 368));
    route.press_x(); // Third throw: second hit.
    route.frames(240, 0);
    assert!(flag(&route, 0x292), "second pot opens the blue door");
    assert!(flag(&route, 0x2f) && !flag(&route, 0x2e));
    route.finish_scene(5000);
    assert!(route.world().patched_cells().contains(&(11, 21, 0xcb)));
    assert!(route.world().patched_cells().contains(&(11, 20, 0xf6)));
    assert!(!route.world().pad_locked());
    assert!(route.world().pot().is_none());
    assert!(route
        .world()
        .residents()
        .iter()
        .all(|resident| resident.record != 0x03_8c32));
    // The opened stairs must be enterable, not merely drawn differently.
    route.legs(&[(30, U), (400, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x0e, (152, 880)),
        "Ark walks through the opened blue door"
    );
    route.legs(&[(33, L), (80, 0), (35, U), (400, 0)]);
    assert_eq!(route.world().map(), 0x20, "the post-door route continues");
    route
}

fn european_rom(trace_env: &str) -> Option<Vec<u8>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../local/Terranigma (E) [!].smc"
    );
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && std::env::var_os(trace_env).is_none() =>
        {
            None // The owned ROM is optional for ordinary tests.
        }
        Err(error) => panic!("cannot read European ROM for {trace_env}: {error}"),
    }
}

#[test]
fn fresh_european_retry_and_separate_pot_presses_open_the_blue_door() {
    let Some(bytes) = european_rom("CRYSTA_WEB_EU_TRACE") else {
        return;
    };
    opened_blue_door(&bytes, "CRYSTA_WEB_EU_TRACE").save_trace();
}

#[test]
#[allow(clippy::too_many_lines)] // Checkpoints follow the same live host through the detour.
fn fresh_european_post_box_frozen_town_detour() {
    let Some(bytes) = european_rom("CRYSTA_WEB_EU_DETOUR_TRACE") else {
        return;
    };
    let mut route = opened_blue_door(&bytes, "CRYSTA_WEB_EU_DETOUR_TRACE");
    route.legs(&[(33, L), (80, 0), (35, U), (400, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (136, 128))
    );
    assert!(!flag(&route, 0x22));
    assert!(route.world().dialogue().is_some(), "the box's entry voice");
    route.finish_scene(4000);
    route.legs(&[(150, D), (180, 0)]);
    assert_eq!(route.world().position(), (136, 351));
    route.legs(&[(25, D), (180, 0)]);
    assert_eq!(
        route.world().position(),
        (136, 359),
        "first contact recoils"
    );
    assert!(!flag(&route, 0x22));
    assert!(route.world().dialogue().is_some(), "the box warns Ark");
    route.finish_scene(4000);
    route.legs(&[(15, D), (300, 0)]);
    assert!(flag(&route, 0x22), "the second approach opens the box");
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (136, 368))
    );

    let mut landings = Vec::new();
    for frame in 0..16000 {
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        let before = route.world().map();
        route.frame(if confirm { X } else { 0 });
        if route.world().map() != before {
            landings.push((route.world().map(), route.world().position()));
        }
        if flag(&route, 0x244) && !route.world().pad_locked() && !route.world().in_transition() {
            break;
        }
    }
    assert_eq!(
        landings,
        [
            (0x41, (136, 208)),
            (0x44, (392, 464)),
            (0x42, (136, 464)),
            (0x43, (392, 208)),
            (0x41, (136, 208)),
        ]
    );
    assert!(flag(&route, 0x243) && flag(&route, 0x244));
    assert!(!route.world().pad_locked() && !route.world().in_scene());
    assert!(route.world().dialogue().is_none());
    route.legs(&[(12, L), (120, 0), (12, U), (120, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x41, (120, 192))
    );

    // Reach the wooden arch and spear on foot. Both conversations need
    // separate X edges; the second actually takes the Crystal Spear.
    route.legs(&[
        (8, L),
        (12, 0),
        (32, U),
        (12, 0),
        (32, U),
        (12, 0),
        (8, L),
        (12, 0),
        (32, U),
        (12, 0),
    ]);
    assert_eq!(route.world().position(), (72, 80), "at the weapon arch");
    route.press_x();
    route.frames(180, 0);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x42, (136, 464))
    );
    route.legs(&[
        (44, L),
        (12, 0),
        (60, U),
        (12, 0),
        (32, L),
        (12, 0),
        (32, U),
        (12, 0),
        (20, R),
        (12, 0),
        (20, U),
        (12, 0),
    ]);
    assert_eq!(route.world().position(), (72, 384), "at the spear");
    assert_eq!(route.world().facing(), Direction::Up);
    for id in [0x240, 0x241, 0x242] {
        assert!(!flag(&route, id), "spear event {id:#x} not granted yet");
    }
    route.press_x();
    assert!(flag(&route, 0x240), "first talk with the spear");
    assert!(!flag(&route, 0x241) && !flag(&route, 0x242));
    assert!(route.finish_scene(5000), "consent to take the spear");
    assert!(flag(&route, 0x241) && !flag(&route, 0x242));
    route.press_x();
    assert!(flag(&route, 0x242), "second talk takes the spear");
    for frame in 0..6000 {
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        route.frame(if confirm { X } else { 0 });
        if route.world().map() == 0x21 {
            break;
        }
    }
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (136, 368))
    );
    assert!(
        route.world().items().contains(&0x81),
        "actual Crystal Spear inventory"
    );

    // Frozen return: its authenticated player stream descends through collision;
    // release precedes the retained manual Left and Up boundaries.
    for frame in 0..12000 {
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        route.frame(if confirm { X } else { 0 });
        if flag(&route, 0x23) && !route.world().pad_locked() && route.world().dialogue().is_none() {
            break;
        }
    }
    assert!(
        flag(&route, 0xfe) && flag(&route, 0x23),
        "portable frozen-return flags must precede manual input"
    );
    assert!(
        !route.world().pad_locked() && route.world().dialogue().is_none(),
        "portable frozen return releases the pad"
    );
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (136, 464)),
        "portable control releases at the native scripted landing"
    );
    route.legs(&[(12, L), (12, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (120, 464)),
        "manual Left resumes from the scripted release position"
    );
    route.legs(&[(12, U), (12, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x21, (120, 448)),
        "retained manual Up follows the native boundary"
    );
    route.legs(&[(12, R), (12, 0)]);
    assert_eq!(
        route.world().position(),
        (136, 448),
        "return to the stair lane"
    );
    for (map, landing) in [(0x20, (360, 880)), (0x0e, (104, 880)), (0x0c, (184, 368))] {
        if map != 0x20 {
            let x = if map == 0x0e { 408 } else { 152 };
            route.until(200, R, |r| r.world().position().0 >= x);
            route.frames(12, 0);
        }
        route.until(400, U, |r| r.world().map() == map);
        route.until(200, 0, |r| !r.world().in_transition());
        assert_eq!(
            (route.world().map(), route.world().position()),
            (map, landing)
        );
    }
    assert!(flag(&route, 0x23) && !flag(&route, 0x21));

    // Frozen neighbor has its own European page. The room Elder is absent
    // while $27 is set and $21 is not; the doorway Elder is a different actor.
    route.legs(&[(34, D), (12, 0), (44, L), (12, 0)]);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x0c, (120, 417))
    );
    route.until(35, D, |r| r.world().position().1 >= 432);
    assert_eq!(route.world().position(), (120, 432));
    route.frames(160, R);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x10, (321, 432))
    );
    assert!(route
        .world()
        .residents()
        .iter()
        .any(|r| r.record == 0x03_8d84 && !r.hidden));
    route.until(200, R, |r| r.world().position().0 >= 424);
    assert_eq!(route.world().position(), (424, 432));
    route.legs(&[(20, 0), (1, U)]);
    assert_eq!(route.world().facing(), Direction::Up);
    assert!(
        route.world().dialogue().is_none(),
        "no preexisting neighbor page"
    );
    route.press_x();
    assert_eq!(
        route
            .world()
            .dialogue()
            .map(|view| view.page.boundary_source()),
        Some(0x88_e1ea),
        "frozen neighbor's European page"
    );
    route.finish_scene(4000);
    assert!(!route.world().pad_locked() && !flag(&route, 0x21));

    route.until(500, L, |r| r.world().map() == 0x0c);
    route.frames(100, 0);
    assert_eq!(route.world().position(), (215, 432));
    route.until(180, L, |r| r.world().position().0 <= 136);
    route.frames(12, 0);
    assert_eq!(route.world().position(), (136, 432));
    route.until(320, U, |r| r.world().position().1 <= 352);
    assert_eq!(
        (route.world().map(), route.world().position()),
        (0x0c, (136, 352))
    );
    route.frame(U | X); // Open B's doorway with a fresh X edge.
    route.frames(130, 0);
    assert!(route.world().patched_cells().contains(&(8, 20, 0xf7)));
    route.until(320, U, |r| r.world().map() == 0x0b);
    route.frames(130, 0);
    assert_eq!(route.world().position(), (120, 191));
    assert!(flag(&route, 0x27) && !flag(&route, 0x21));
    assert!(
        route
            .world()
            .residents()
            .iter()
            .all(|r| r.record != 0x03_8b9e),
        "room Elder is absent while $27 XOR $21"
    );
    assert!(!flag(&route, 0x296));

    route.until(550, D, |r| r.world().map() == 0x0c);
    route.frames(130, 0);
    assert_eq!(route.world().position(), (136, 353));
    route.until(150, D, |r| r.world().position().1 >= 417);
    route.frames(12, 0);
    route.until(100, L, |r| r.world().position().0 <= 120);
    route.frames(12, 0);
    route.until(200, D, |r| r.world().map() == 0x0d);
    route.frames(130, 0);
    assert_eq!(route.world().position(), (120, 625));
    assert!(route
        .world()
        .residents()
        .iter()
        .any(|r| r.record == 0x03_8cc6));
    route.until(300, D, |r| r.world().position().1 >= 704);
    assert_eq!(route.world().position(), (120, 704));
    assert!(!flag(&route, 0x21) && !flag(&route, 0x296));
    route.press_x();
    assert!(
        flag(&route, 0x21) && !flag(&route, 0x296),
        "doorway Elder learns of frozen Crysta"
    );
    for frame in 0..6000 {
        if route
            .world()
            .dialogue()
            .and_then(|page| page.cursor)
            .is_some()
        {
            break;
        }
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        route.frame(if confirm { X } else { 0 });
    }
    assert!(
        route
            .world()
            .dialogue()
            .and_then(|page| page.cursor)
            .is_some(),
        "Elder's mission choice"
    );
    assert!(
        flag(&route, 0x21) && !flag(&route, 0x296),
        "mission follows the answer"
    );
    assert!(route.finish_scene(6000), "accepted doorway Elder's mission");
    assert!(flag(&route, 0x296) && !route.world().pad_locked());
    assert!(!flag(&route, 0x3c), "town scene has not run yet");

    route.until(250, D, |r| r.world().map() == 0x0a);
    assert!(!flag(&route, 0x3c), "town arrival precedes its scene");
    let mut saw_town_page = route.world().dialogue().is_some();
    for frame in 0..6000 {
        let confirm =
            route.world().dialogue().is_some() && !route.world().typing() && frame % 2 == 0;
        route.frame(if confirm { X } else { 0 });
        saw_town_page |= route.world().dialogue().is_some();
        if flag(&route, 0x3c) && !route.world().pad_locked() && route.world().dialogue().is_none() {
            break;
        }
    }
    assert!(saw_town_page, "town scene opened a European page");
    assert!(flag(&route, 0x3c), "town scene after detour");
    assert!(!route.world().pad_locked() && !route.world().in_scene());
    assert!(route.world().dialogue().is_none());
    route.frames(48, 0); // Run the town controller's deferred target.
    assert!(route.world().frozen_scripts().is_empty());
    assert_eq!(route.world().player_script_frozen_at(), None);
    route.until(450, D, |r| r.world().map() == 0x03);
    route.until(200, 0, |r| !r.world().in_transition());
    for step in 1..=16 {
        route.frame(0);
        assert_eq!(
            route.world().position(),
            (536, 528 + step),
            "neutral map arrival"
        );
    }
    for id in [
        0x20, 0x21, 0x22, 0x23, 0x26, 0x27, 0x28, 0x3c, 0xfe, 0x240, 0x241, 0x242, 0x243, 0x244,
        0x292, 0x296,
    ] {
        assert!(flag(&route, id), "EU detour flag {id:#x}");
    }
    assert!(
        flag(&route, 0x2f) && !flag(&route, 0x2e),
        "retry, not direct acceptance"
    );
    assert!(route.world().items().contains(&0x81));
    assert!(!route.world().pad_locked() && !route.world().in_scene());
    assert!(route.world().dialogue().is_none());
    route.legs(&[(16, D), (16, 0)]);
    assert_eq!(route.world().map(), 0x03);
    assert!(
        route.world().position().1 > 544,
        "Ark walks after the detour"
    );
    route.save_trace();
}

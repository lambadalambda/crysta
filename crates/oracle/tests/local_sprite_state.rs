//! Optional input-only sprite hardware inspection. One oracle boot per process.
use oracle::{export::digest_hex, Button, Session};
use std::{io::Write, path::Path, process::Command};

#[path = "../../../tools/new-game-qualification/bootstrap.rs"]
mod bootstrap;

#[test]
fn fresh_sprite_reads_are_repeatable_and_do_not_mutate_reference_state() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Tenchi Souzou (Japan).sfc");
    if !path.exists() {
        eprintln!("SKIP: optional Japanese ROM absent");
        return;
    }
    if std::env::var_os("ORACLE_SPRITE_CHILD").is_some() {
        run_child(&path);
    }
    let run = || {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "fresh_sprite_reads_are_repeatable_and_do_not_mutate_reference_state",
                "--nocapture",
            ])
            .env("ORACLE_SPRITE_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout)
            .unwrap()
            .lines()
            .find(|line| line.starts_with("sprite-state "))
            .unwrap()
            .to_owned()
    };
    let first = run();
    assert_eq!(first, "sprite-state frame=6800 oam=eb7269c9cfae9312395d4450d732e6f27d68a8bf9de453c8067d30255e0ccf5b obsel=2 first=0");
    assert_eq!(
        first,
        run(),
        "two independent fresh input-only boots must agree"
    );
    eprintln!("{first}");
}

fn run_child(path: &Path) -> ! {
    let rom = rom::Rom::load(&std::fs::read(path).unwrap()).unwrap();
    let mut session = Session::new(&rom).unwrap();
    for frame in 0..6800 {
        for button in [Button::Start, Button::Down, Button::A] {
            session.set_button(
                button,
                bootstrap::INPUTS
                    .iter()
                    .any(|&(b, start, end)| b == button && (start..end).contains(&frame)),
            );
        }
        session.run_frame();
    }
    // save_state synchronizes the emulator and is not a passive observation.
    // Compare all existing read-only exported surfaces instead.
    let observe = || {
        (
            session.frame_state(),
            session.cpu_registers(),
            digest_hex(&session.wram_image()),
            digest_hex(
                &session
                    .vram()
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            ),
            digest_hex(
                &session
                    .cgram()
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            ),
            digest_hex(&session.apu_ram()),
            digest_hex(session.pixels()),
            digest_hex(
                &session
                    .samples()
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            ),
        )
    };
    let before = observe();
    let sprite = session.sprite_state();
    assert_eq!(sprite.oam.len(), 544);
    assert!(sprite.first_sprite < 128);
    assert!(sprite.oam.iter().any(|&byte| byte != 0));
    assert_eq!(sprite, session.sprite_state());
    assert_eq!(
        before,
        observe(),
        "read-only inspection changed an exported state surface"
    );
    println!(
        "sprite-state frame=6800 oam={} obsel={} first={}",
        digest_hex(&sprite.oam),
        sprite.obsel,
        sprite.first_sprite
    );
    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}

//! Optional owned European-ROM witness; ares can boot only once per process.
use oracle::{Session, MAX_APU_PORT_WRITES};
use std::{path::Path, process::Command};

#[test]
fn native_startup_apu_port_writes_and_quiet_interval() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/Terranigma (E) [!].smc");
    if !path.exists() {
        eprintln!("skipping: local European dump not present");
        return;
    }
    if std::env::var_os("ORACLE_APU_ROM_CHILD").is_some() {
        let image = std::fs::read(&path).expect("owned ROM");
        let rom = rom::Rom::load(&image).expect("validated owned ROM");
        assert_eq!(rom.revision(), rom::Revision::EuropeEnglish);
        let mut session = Session::new(&rom).expect("native session");
        let mut observed = Vec::new();
        for _ in 0..120 {
            session.run_frame();
            let capture = session.take_apu_port_writes();
            assert!(!capture.overflow, "startup write buffer overflowed");
            assert!(capture.entries.len() <= MAX_APU_PORT_WRITES);
            observed.extend(capture.entries);
        }
        assert!(!observed.is_empty(), "startup must write CPU-to-APU ports");
        assert!(observed.iter().all(|w| w.port < 4));
        assert!(observed
            .windows(2)
            .all(|pair| pair[0].frame <= pair[1].frame));
        session.clear_apu_port_writes();
        // No execution between clear and take: this is an intentionally quiet interval.
        let quiet = session.take_apu_port_writes();
        assert!(quiet.entries.is_empty() && !quiet.overflow);
        std::process::exit(0);
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native_startup_apu_port_writes_and_quiet_interval",
            "--nocapture",
        ])
        .env("ORACLE_APU_ROM_CHILD", "1")
        .output()
        .expect("fresh oracle child");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

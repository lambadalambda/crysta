//! Guard the deliberately dependency-free portable boundary.
#[test]
fn core_stays_no_std_and_has_no_platform_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("dependencies"),
        "review the portable boundary before adding dependencies"
    );
    assert!(include_str!("../src/lib.rs").starts_with("#![no_std]"));
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(
                !text.contains("extern crate std") && !text.contains("std::"),
                "host API in {}",
                path.display()
            );
        }
    }
}

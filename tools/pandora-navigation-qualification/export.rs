fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "export ROM local/NEW-OUTPUT");
    let out = std::path::Path::new(&args[2]);
    assert!(
        out.starts_with("local")
            && !out
                .components()
                .any(|c| c == std::path::Component::ParentDir)
    );
    let image = std::fs::read(&args[1]).unwrap();
    let data = pandora_navigation::compile(&image).unwrap();
    std::fs::create_dir(out).unwrap();
    for p in data.profiles() {
        let bytes: Vec<_> = p
            .room()
            .cells()
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        std::fs::write(out.join(format!("{}.grid", p.name())), bytes).unwrap();
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&data.metadata()).unwrap(),
    )
    .unwrap();
}

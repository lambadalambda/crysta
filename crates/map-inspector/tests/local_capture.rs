//! Optional end-to-end capture qualification, using a fresh oracle process.
use std::{collections::BTreeSet, fmt::Write as _, path::Path, process::Command};

const PREDECESSOR: &str =
    include_str!("../../../tools/map-inspector-qualification/current-producer.json");
const PREDECESSOR_BRIDGE: &str =
    include_str!("../../../tools/map-inspector-qualification/producer-bridge.json");
const LIBRARY: &str =
    include_str!("../../../tools/map-inspector-qualification/library-producer.json");
const LIBRARY_BRIDGE: &str =
    include_str!("../../../tools/map-inspector-qualification/library-producer-bridge.json");
const OBSERVER: &str = include_str!("../../../tools/map-inspector-qualification/observer.json");
const MIGRATION: &str = include_str!("../../../tools/map-inspector-qualification/migration.json");
const OBSERVER_SHA: &str = "7fabf5688943eca89c43ad5aee02d348187fc3491296b1c401535553fbe3a718";
const MIGRATION_SHA: &str = "db249179718cb6bcf9c1755093d441d0094dd39fc836d079220defd3b289ab3c";
const OLD_MAIN_SHA: &str = "7736b543c442e6e4c2789fb13f6f177d1335e78f11810d5023a49c313b27a4d3";
const PREDECESSOR_SHA: &str = "85de8d72d6f0a433345645f5dd86f5c80f8e1ffd18549fb357a59b2d97590714";
const PREDECESSOR_BRIDGE_SHA: &str =
    "46a2b7fda7525c8c7da664b83ec182160b39f4a67d8d0e958c573cea606795c0";
const LIBRARY_SHA: &str = "00298d9350a143abeb83bb95ae093feba81d6c9850ab4722bf015834d88f6143";
const LIBRARY_BRIDGE_SHA: &str = "18cfd3ec329e70159d3ad7613dd73f826d03b55c573274661337f9277060b75d";
const REPIN: &str = include_str!("../../../tools/map-inspector-qualification/repin-producer.json");
const REPIN_BRIDGE: &str =
    include_str!("../../../tools/map-inspector-qualification/repin-producer-bridge.json");
const REPIN_SHA: &str = "e58a0f232a8ce9cc86186e515a9156ca32c4fd992c4a7f38cd859c117a247c33";
const REPIN_BRIDGE_SHA: &str = "5fed82d65b682ca001dfa55b3ce6f9c60f9911d7b9cd0313e461e688b167173f";
const REPIN_FILES: &[&str] = &["crates/map-inspector/src/main.rs"];
const REPIN_FIELDS: &[&str] = &[
    "schema_version",
    "kind",
    "epoch",
    "policy",
    "original_descriptor_sha256",
    "migration_sha256",
    "predecessor_descriptor_sha256",
    "predecessor_bridge_sha256",
    "replaced_source_hashes",
];
const MAIN: &str = "crates/map-inspector/src/main.rs";
// The rustfmt declaration-order repair, pinned as an exact byte delta so the
// reformatted producer still reconstructs to OLD_MAIN_SHA.
const PINNED_MODS: &[u8] = b"mod house_progression;\nmod house_profiles;\n";
const FORMATTED_MODS: &[u8] = b"mod house_profiles;\nmod house_progression;\n";
const ARCHIVE: &str = include_str!(
    "../../../tools/map-inspector-qualification/epochs/threaded-video-v0/reference.json"
);
const SOURCE_FILES: &[&str] = &[
    "Cargo.lock",
    "crates/map-inspector/Cargo.toml",
    "crates/map-inspector/src/main.rs",
    "crates/map-inspector/web/viewer.html",
    "crates/oracle/build.rs",
    "crates/oracle/src/lib.rs",
    "vendor/ares/ares-unity.cpp",
    "vendor/ares/shims.cpp",
    "vendor/ares/ares/ares/ares.hpp",
    "vendor/ares/ares/ares/node/video/screen.cpp",
    "vendor/ares/ares/sfc/ppu/main.cpp",
    "vendor/ares/ares/sfc/ppu/color.cpp",
    "vendor/ares/ares/sfc/system/serialization.cpp",
];

// Separate reviewed preview hooks; the historical 13-file inventory above is unchanged.
const ADDITIONAL_FILES: &[&str] = &[
    "crates/map-inspector/src/pandora_navigation.rs",
    "crates/map-inspector/src/pandora_progression.rs",
    "crates/map-inspector/src/room_art.rs",
    "crates/map-inspector/src/room_art/backgrounds.rs",
    "crates/map-inspector/src/room_art/carry.rs",
    "crates/map-inspector/src/room_art/door.rs",
    "crates/map-inspector/src/room_art/pandora.rs",
    "crates/map-inspector/src/room_art/world_patches.rs",
    "crates/map-inspector/src/room_camera.rs",
    "crates/map-inspector/src/room_preview.rs",
    "crates/map-inspector/src/room_server.rs",
    "crates/map-inspector/web/room-slice.html",
];

const REPLACED_FILES: &[&str] = &[
    "Cargo.lock",
    "crates/map-inspector/Cargo.toml",
    "crates/map-inspector/src/room_preview.rs",
    "crates/map-inspector/web/room-slice.html",
];
const LIBRARY_FILES: &[&str] = &[
    "crates/map-inspector/src/lib.rs",
    "crates/map-inspector/src/static_background.rs",
    "crates/map-inspector/src/visual_export.rs",
];
const QUALIFICATION_FILES: &[&str] = &["crates/map-inspector/tests/public_preview.rs"];
const ADAPTER_FILES: &[&str] = &[
    "Cargo.toml",
    "crates/pandora-web/Cargo.toml",
    "crates/pandora-web/examples/parity.rs",
    "crates/pandora-web/src/lib.rs",
    "crates/pandora-web/tests/session.rs",
    "tools/pandora-preview/README.md",
    "tools/pandora-preview/bootstrap.mjs",
    "tools/pandora-preview/bootstrap.test.mjs",
    "tools/pandora-preview/build.sh",
    "tools/pandora-preview/main.mjs",
    "tools/pandora-preview/parity-actions.json",
    "tools/pandora-preview/parity.mjs",
    "tools/pandora-preview/runtime-loader.js",
    "tools/pandora-preview/worker.mjs",
];
const LIBRARY_FIELDS: &[&str] = &[
    "schema_version",
    "kind",
    "epoch",
    "policy",
    "original_descriptor_sha256",
    "migration_sha256",
    "predecessor_descriptor_sha256",
    "predecessor_bridge_sha256",
    "replaced_source_hashes",
    "library_source_hashes",
    "qualification_source_hashes",
    "adapter_source_hashes",
];

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            write!(&mut hex, "{byte:02x}").expect("writing to a String");
            hex
        })
}

fn check_inventory(hashes: &serde_json::Value, files: &[&str]) {
    assert_eq!(
        hashes
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        files.iter().copied().collect::<BTreeSet<_>>(),
        "producer source inventory"
    );
}

fn check_source_inventory(observer: &serde_json::Value) {
    check_inventory(&observer["source_hashes"], SOURCE_FILES);
}

#[test]
fn incomplete_observer_sources_are_rejected() {
    let observer: serde_json::Value = serde_json::from_str(OBSERVER).unwrap();
    for name in SOURCE_FILES {
        let mut changed = observer.clone();
        changed["source_hashes"]
            .as_object_mut()
            .unwrap()
            .remove(*name);
        assert!(std::panic::catch_unwind(|| check_source_inventory(&changed)).is_err());
    }
    let mut empty = observer;
    empty["source_hashes"] = serde_json::json!({});
    assert!(std::panic::catch_unwind(|| check_source_inventory(&empty)).is_err());
}

fn check_producer_identity(current: &serde_json::Value) {
    assert_eq!(sha256(OBSERVER.as_bytes()), OBSERVER_SHA);
    assert_eq!(sha256(MIGRATION.as_bytes()), MIGRATION_SHA);
    let original: serde_json::Value = serde_json::from_str(OBSERVER).unwrap();
    assert_eq!(current["schema_version"], 1);
    assert_eq!(current["epoch"], "headless-sync-video-v1");
    assert_eq!(current["epoch"], original["epoch"]);
    assert_eq!(current["policy"], original["policy"]);
    assert_eq!(current["original_descriptor_sha256"], OBSERVER_SHA);
    assert_eq!(current["migration_sha256"], MIGRATION_SHA);
    check_source_inventory(current);
    check_inventory(&current["additional_source_hashes"], ADDITIONAL_FILES);
    for name in SOURCE_FILES.iter().copied().filter(|name| *name != MAIN) {
        assert_eq!(
            current["source_hashes"][name],
            original["source_hashes"][name]
        );
    }
}

#[test]
fn historical_identity_substitution_is_rejected() {
    let predecessor: serde_json::Value = serde_json::from_str(PREDECESSOR).unwrap();
    check_producer_identity(&predecessor);
    for field in [
        "schema_version",
        "epoch",
        "policy",
        "original_descriptor_sha256",
        "migration_sha256",
    ] {
        let mut changed = predecessor.clone();
        changed[field] = serde_json::json!("substitution");
        assert!(std::panic::catch_unwind(|| check_producer_identity(&changed)).is_err());
    }
    let old = serde_json::from_str(OBSERVER).unwrap();
    assert!(std::panic::catch_unwind(|| check_producer_identity(&old)).is_err());
}

fn positions(source: &[u8], find: &[u8]) -> Vec<usize> {
    source
        .windows(find.len())
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == find).then_some(index))
        .collect()
}

fn count(source: &[u8], find: &[u8]) -> usize {
    positions(source, find).len()
}

fn replace_exactly_once(source: &[u8], find: &[u8], replace: &[u8], what: &str) -> Vec<u8> {
    let found = positions(source, find);
    assert_eq!(found.len(), 1, "exact {what} anchor required");
    let index = found[0];
    [&source[..index], replace, &source[index + find.len()..]].concat()
}

fn check_main_registration(main: &[u8]) {
    const ANCHOR: &[u8] = b"mod opening_qualification;\n";
    const INSERT: &[u8] = b"pub mod pandora_navigation;\npub mod pandora_progression;\n";
    // Undo ONLY the two exact deltas that separate this file from the old
    // producer -- the rustfmt module reorder and the anchored registration
    // insertion -- then authenticate every remaining byte. No line stripping
    // or whitespace normalization.
    let ordered = replace_exactly_once(main, FORMATTED_MODS, PINNED_MODS, "module-order");
    let old = replace_exactly_once(&ordered, &[ANCHOR, INSERT].concat(), ANCHOR, "registration");
    assert_eq!(sha256(&old), OLD_MAIN_SHA, "non-registration main change");
    // Injectivity: the reconstruction is only a proof if the old blob offers a
    // single site for each delta, so no second main.rs can reconstruct to it.
    assert_eq!(
        count(&old, ANCHOR),
        1,
        "ambiguous registration anchor in old main"
    );
    assert_eq!(
        count(&old, PINNED_MODS),
        1,
        "ambiguous module order in old main"
    );
}

#[test]
fn non_registration_main_changes_are_rejected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main = std::fs::read(root.join("crates/map-inspector/src/main.rs")).unwrap();
    check_main_registration(&main);
    let text = std::str::from_utf8(&main).unwrap();
    let pinned_mods = std::str::from_utf8(PINNED_MODS).unwrap();
    let formatted_mods = std::str::from_utf8(FORMATTED_MODS).unwrap();
    for changed in [
        format!("{text}\n"),
        text.replace('\n', "\r\n"),
        text.replace("fn main()", "fn changed()"),
        text.replace("pub mod pandora_navigation;\n", ""),
        text.replace("pub mod pandora_progression;\n", ""),
        text.replace("mod opening_qualification;", "mod opening_qualification; "),
        // The reorder is pinned in its formatted direction, is required, and
        // must stay unambiguous.
        formatted_mods.to_string(),
        text.replace(formatted_mods, pinned_mods),
        text.replace(formatted_mods, ""),
        format!("{text}{formatted_mods}"),
    ] {
        assert!(std::panic::catch_unwind(|| check_main_registration(changed.as_bytes())).is_err());
    }
}

fn predecessor_hash<'a>(predecessor: &'a serde_json::Value, name: &str) -> &'a str {
    predecessor["source_hashes"][name]
        .as_str()
        .or_else(|| predecessor["additional_source_hashes"][name].as_str())
        .expect("predecessor source identity")
}

fn check_library_identity(library: &serde_json::Value, predecessor: &serde_json::Value) {
    use std::collections::BTreeSet;
    assert_eq!(sha256(PREDECESSOR.as_bytes()), PREDECESSOR_SHA);
    assert_eq!(
        sha256(PREDECESSOR_BRIDGE.as_bytes()),
        PREDECESSOR_BRIDGE_SHA
    );
    assert_eq!(sha256(LIBRARY.as_bytes()), LIBRARY_SHA);
    assert_eq!(sha256(LIBRARY_BRIDGE.as_bytes()), LIBRARY_BRIDGE_SHA);
    check_producer_identity(predecessor);
    assert_eq!(
        library
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        LIBRARY_FIELDS.iter().copied().collect::<BTreeSet<_>>()
    );
    assert_eq!(library["schema_version"], 1);
    assert_eq!(library["kind"], "map-inspector-library-producer");
    assert_eq!(library["epoch"], predecessor["epoch"]);
    assert_eq!(library["policy"], predecessor["policy"]);
    assert_eq!(library["original_descriptor_sha256"], OBSERVER_SHA);
    assert_eq!(library["migration_sha256"], MIGRATION_SHA);
    assert_eq!(library["predecessor_descriptor_sha256"], PREDECESSOR_SHA);
    assert_eq!(library["predecessor_bridge_sha256"], PREDECESSOR_BRIDGE_SHA);
    check_inventory(&library["replaced_source_hashes"], REPLACED_FILES);
    check_inventory(&library["library_source_hashes"], LIBRARY_FILES);
    check_inventory(&library["qualification_source_hashes"], QUALIFICATION_FILES);
    check_inventory(&library["adapter_source_hashes"], ADAPTER_FILES);
    for name in REPLACED_FILES {
        check_inventory(
            &library["replaced_source_hashes"][name],
            &["predecessor_sha256", "current_sha256"],
        );
        assert_eq!(
            library["replaced_source_hashes"][name]["predecessor_sha256"],
            predecessor_hash(predecessor, name)
        );
        assert_ne!(
            library["replaced_source_hashes"][name]["current_sha256"],
            library["replaced_source_hashes"][name]["predecessor_sha256"]
        );
    }
    let report: serde_json::Value = serde_json::from_str(LIBRARY_BRIDGE).unwrap();
    assert_eq!(report["library_descriptor_sha256"], LIBRARY_SHA);
    assert_eq!(report["predecessor_descriptor_sha256"], PREDECESSOR_SHA);
    assert_eq!(report["predecessor_bridge_sha256"], PREDECESSOR_BRIDGE_SHA);
    assert_eq!(report["migration_sha256"], MIGRATION_SHA);
    assert_eq!(
        report["nonpixel_manifest_sha256"],
        "7998be259cce4218983f03bb81e3bf189577958dc9f190ba38880036d680cb22"
    );
}

/// The repin stage authenticates the frozen library stage by hash, then records
/// exactly one replacement over it. Unchanged sources stay derived from the
/// frozen descriptors; there is deliberately no field in which to reseal one.
fn check_repin_identity(repin: &serde_json::Value, library: &serde_json::Value) {
    use std::collections::BTreeSet;
    assert_eq!(sha256(REPIN.as_bytes()), REPIN_SHA);
    assert_eq!(sha256(REPIN_BRIDGE.as_bytes()), REPIN_BRIDGE_SHA);
    assert_eq!(
        repin
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        REPIN_FIELDS.iter().copied().collect::<BTreeSet<_>>()
    );
    assert_eq!(repin["schema_version"], 1);
    assert_eq!(repin["kind"], "map-inspector-repin-producer");
    assert_eq!(repin["epoch"], library["epoch"]);
    assert_eq!(repin["policy"], library["policy"]);
    assert_eq!(repin["original_descriptor_sha256"], OBSERVER_SHA);
    assert_eq!(repin["migration_sha256"], MIGRATION_SHA);
    assert_eq!(repin["predecessor_descriptor_sha256"], LIBRARY_SHA);
    assert_eq!(repin["predecessor_bridge_sha256"], LIBRARY_BRIDGE_SHA);
    check_inventory(&repin["replaced_source_hashes"], REPIN_FILES);
    for name in REPIN_FILES {
        check_inventory(
            &repin["replaced_source_hashes"][name],
            &["predecessor_sha256", "current_sha256"],
        );
        assert_ne!(
            repin["replaced_source_hashes"][name]["current_sha256"],
            repin["replaced_source_hashes"][name]["predecessor_sha256"]
        );
        // The declared pre-state must be the frozen stage's actual identity,
        // not a plausible-looking hash. Checked here so a clean checkout, which
        // never runs the Python bridge, still rejects a fabricated predecessor.
        let predecessor: serde_json::Value = serde_json::from_str(PREDECESSOR).unwrap();
        let declared = repin["replaced_source_hashes"][name]["predecessor_sha256"]
            .as_str()
            .expect("declared predecessor identity");
        let frozen = library["replaced_source_hashes"][name]["current_sha256"]
            .as_str()
            .unwrap_or_else(|| predecessor_hash(&predecessor, name));
        assert_eq!(
            declared, frozen,
            "declared pre-state is not the frozen stage"
        );
    }
    check_repin_report(REPIN_BRIDGE, repin);
}

/// A report whose header and body disagree is exactly the defect this stage
/// exists to avoid: every retained envelope must bind to this descriptor.
fn check_repin_report(report: &str, repin: &serde_json::Value) {
    let report: serde_json::Value = serde_json::from_str(report).unwrap();
    assert_eq!(report["repin_descriptor_sha256"], REPIN_SHA);
    assert_eq!(report["predecessor_descriptor_sha256"], LIBRARY_SHA);
    assert_eq!(report["predecessor_bridge_sha256"], LIBRARY_BRIDGE_SHA);
    assert_eq!(report["migration_sha256"], MIGRATION_SHA);
    assert_eq!(
        report["nonpixel_manifest_sha256"],
        "7998be259cce4218983f03bb81e3bf189577958dc9f190ba38880036d680cb22"
    );
    let producers = report["producers"].as_array().expect("retained producers");
    assert_eq!(
        producers.len(),
        2,
        "two isolated producer envelopes required"
    );
    for producer in producers {
        assert_eq!(producer["descriptor_sha256"], REPIN_SHA);
        assert_eq!(producer["kind"], repin["kind"]);
        assert_eq!(
            producer["replaced_source_hashes"],
            repin["replaced_source_hashes"]
        );
        for name in REPIN_FILES {
            assert_eq!(
                producer["source_hashes"][name],
                repin["replaced_source_hashes"][name]["current_sha256"]
            );
        }
    }
    assert_ne!(
        producers[0]["process"]["run_id"],
        producers[1]["process"]["run_id"]
    );
    assert_ne!(producers[0]["target_dir"], producers[1]["target_dir"]);
}

/// Build-input files pinned by projection rather than whole-file hash.
///
/// `Cargo.lock` and the workspace `Cargo.toml` are producer sources because
/// they decide what the producer compiles against. Both also record things the
/// producer cannot reach: adding an unrelated workspace member rewrites
/// `members` and appends a `[[package]]` block without changing a single input
/// to the capture.
///
/// Rather than spend a repin on a provable no-op, each is pinned as a
/// projection, and the chain is anchored on a frozen copy of the exact file the
/// descriptor pinned:
///
/// ```text
/// descriptor entry == sha256(pinned fixture)   descriptor <-> fixture
/// projection(fixture) == projection(live)      fixture    <-> live
/// ```
///
/// There is no free-floating constant to edit: the fixture is a real file whose
/// whole-file hash the descriptor already names.
const LOCK: &str = "Cargo.lock";
const ROOT_MANIFEST: &str = "Cargo.toml";
const PINNED_LOCK: &str =
    include_str!("../../../tools/map-inspector-qualification/pinned/Cargo.lock");
const PINNED_MANIFEST: &str =
    include_str!("../../../tools/map-inspector-qualification/pinned/Cargo.toml");

/// One `[[package]]` block.
///
/// Identity is `(name, version, source)`. Keying on `(name, version)` alone is
/// not enough: cargo emits a dependency reference's source exactly when name
/// and version are ambiguous, which is what a `[patch]` at a git fork that kept
/// its version number produces. Dropping the source there would merge two
/// distinct packages, and the second one's edges would never be walked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LockPackage {
    name: String,
    version: String,
    source: String,
    checksum: String,
    dependencies: Vec<String>,
}

impl LockPackage {
    fn identity(&self) -> (String, String, String) {
        (self.name.clone(), self.version.clone(), self.source.clone())
    }
}

/// Parses the `[[package]]` blocks of a `Cargo.lock`.
fn lockfile_packages(lock: &str) -> Vec<LockPackage> {
    let mut packages = Vec::new();
    let mut current: Option<LockPackage> = None;
    let mut in_dependencies = false;
    let unquote = |line: &str| {
        line.split_once('=')
            .map(|(_, value)| value.trim().trim_matches('"').to_string())
            .unwrap_or_default()
    };
    for line in lock.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            packages.extend(current.take());
            current = Some(LockPackage {
                name: String::new(),
                version: String::new(),
                source: String::new(),
                checksum: String::new(),
                dependencies: Vec::new(),
            });
            in_dependencies = false;
            continue;
        }
        let Some(package) = current.as_mut() else {
            continue;
        };
        if in_dependencies {
            if trimmed == "]" {
                in_dependencies = false;
            } else {
                package
                    .dependencies
                    .push(trimmed.trim_end_matches(',').trim_matches('"').to_string());
            }
            continue;
        }
        match trimmed {
            _ if trimmed.starts_with("name =") => package.name = unquote(trimmed),
            _ if trimmed.starts_with("version =") => package.version = unquote(trimmed),
            _ if trimmed.starts_with("source =") => package.source = unquote(trimmed),
            _ if trimmed.starts_with("checksum =") => package.checksum = unquote(trimmed),
            "dependencies = [" => in_dependencies = true,
            // A new top-level table ends the package blocks.
            _ if trimmed.starts_with('[') => {
                packages.extend(current.take());
                break;
            }
            _ => {}
        }
    }
    packages.extend(current);
    packages
}

/// Splits a dependency reference into its name, optional version and optional
/// source: `"name"`, `"name version"` or `"name version (source)"`.
fn dependency_reference(reference: &str) -> (&str, Option<&str>, Option<&str>) {
    let (head, source) = match reference.split_once(" (") {
        Some((head, rest)) => (head, Some(rest.trim_end_matches(')'))),
        None => (reference, None),
    };
    let mut fields = head.split_whitespace();
    (fields.next().unwrap_or_default(), fields.next(), source)
}

/// Canonical text of `root`'s transitive dependency closure.
fn lockfile_subtree(lock: &str, root: &str) -> String {
    let packages = lockfile_packages(lock);
    let mut wanted: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut queue: Vec<String> = vec![root.to_string()];
    while let Some(reference) = queue.pop() {
        let (name, version, source) = dependency_reference(&reference);
        for package in packages.iter().filter(|package| {
            // A reference that omits a field is ambiguous only when several
            // packages share what it does give; taking all of them can only
            // widen the closure.
            package.name == name
                && version.is_none_or(|version| package.version == version)
                && source.is_none_or(|source| package.source == source)
        }) {
            if wanted.insert(package.identity()) {
                queue.extend(package.dependencies.iter().cloned());
            }
        }
    }
    let mut canonical = String::new();
    for identity in &wanted {
        let Some(package) = packages
            .iter()
            .find(|package| package.identity() == *identity)
        else {
            continue;
        };
        let mut dependencies = package.dependencies.clone();
        dependencies.sort();
        write!(
            canonical,
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1e}",
            package.name,
            package.version,
            package.source,
            package.checksum,
            dependencies.join("\u{1d}")
        )
        .expect("writing to a String cannot fail");
    }
    canonical
}

/// The manifest with only `[workspace] members` elided.
///
/// The producer is built by a pinned command, `cargo build --locked -p
/// map-inspector`. That selects one package, so feature resolution covers only
/// map-inspector's own graph and the *set* of other workspace members cannot
/// reach it. Everything else stays byte-for-byte: `resolver`, `default-members`,
/// `exclude`, `[workspace.package]`, `[workspace.lints]`,
/// `[workspace.dependencies]`, `[patch]` and any profile table.
///
/// This holds for the default resolver behaviour. `resolver.feature-unification
/// = "workspace"` would unify features across all members regardless of `-p`
/// and break the argument; no cargo configuration sets it, and none is pinned,
/// so it is a stated assumption rather than a guarantee.
///
/// The elision is bounded by the `[workspace]` table and by real bracket depth,
/// counted outside quoted strings. A line-prefix match with a trailing-bracket
/// heuristic is not enough -- `members = [...] #` would run the elision on to
/// the next bracketed line and swallow whatever sat between.
fn manifest_without_members(manifest: &str) -> String {
    let mut out = String::new();
    let mut table = String::new();
    let mut depth = 0usize;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if depth > 0 {
            depth = bracket_depth(line, depth);
            continue;
        }
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            table = trimmed.trim_matches(|c| c == '[' || c == ']').to_string();
        }
        let key = trimmed.split_once('=').map(|(key, _)| key.trim());
        if table == "workspace" && key == Some("members") {
            out.push_str("members = <elided>\n");
            depth = bracket_depth(line, 0);
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Bracket nesting after `line`, starting from `depth`, ignoring quoted text.
fn bracket_depth(line: &str, depth: usize) -> usize {
    let mut depth = depth;
    let mut quoted = false;
    for byte in line.bytes() {
        match byte {
            b'"' => quoted = !quoted,
            b'[' if !quoted => depth += 1,
            b']' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn check_library_sources(root: &Path, library: &serde_json::Value) {
    let predecessor: serde_json::Value = serde_json::from_str(PREDECESSOR).unwrap();
    let repin: serde_json::Value = serde_json::from_str(REPIN).unwrap();
    check_library_identity(library, &predecessor);
    check_repin_identity(&repin, library);
    for field in ["source_hashes", "additional_source_hashes"] {
        for (name, expected) in predecessor[field].as_object().unwrap() {
            // Latest stage wins: repin over library over frozen predecessor.
            let expected = repin["replaced_source_hashes"][name]["current_sha256"]
                .as_str()
                .or_else(|| library["replaced_source_hashes"][name]["current_sha256"].as_str())
                .unwrap_or_else(|| expected.as_str().unwrap());
            let contents = std::fs::read(root.join(name)).expect("read predecessor source");
            if name == LOCK {
                // The descriptor names the frozen fixture...
                assert_eq!(
                    sha256(PINNED_LOCK.as_bytes()),
                    expected,
                    "the lockfile's descriptor identity was substituted"
                );
                // ...and the live file must project onto it.
                assert_eq!(
                    lockfile_subtree(std::str::from_utf8(&contents).unwrap(), "map-inspector"),
                    lockfile_subtree(PINNED_LOCK, "map-inspector"),
                    "map-inspector's resolved dependencies changed; \
                     explicitly revalidate before repinning"
                );
                continue;
            }
            assert_eq!(
                sha256(&contents),
                expected,
                "library producer source changed: {name}; explicitly revalidate before repinning"
            );
        }
    }
    for field in [
        "library_source_hashes",
        "qualification_source_hashes",
        "adapter_source_hashes",
    ] {
        for (name, expected) in library[field].as_object().unwrap() {
            let contents = std::fs::read(root.join(name)).expect("read library source");
            if name == ROOT_MANIFEST {
                assert_eq!(
                    sha256(PINNED_MANIFEST.as_bytes()),
                    expected.as_str().unwrap(),
                    "the manifest's descriptor identity was substituted"
                );
                assert_eq!(
                    manifest_without_members(std::str::from_utf8(&contents).unwrap()),
                    manifest_without_members(PINNED_MANIFEST),
                    "the workspace manifest changed outside its member list; \
                     explicitly revalidate before repinning"
                );
                continue;
            }
            assert_eq!(
                sha256(&contents),
                expected.as_str().unwrap(),
                "library producer source changed: {name}; explicitly revalidate before repinning"
            );
        }
    }
    check_main_registration(&std::fs::read(root.join(MAIN)).unwrap());
}

#[test]
fn fixture_library_producer_sources_match() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let library = serde_json::from_str(LIBRARY).unwrap();
    check_library_sources(&root, &library);
}

#[test]
fn library_identity_inventory_and_delta_mutations_are_rejected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let library: serde_json::Value = serde_json::from_str(LIBRARY).unwrap();
    for field in LIBRARY_FIELDS {
        let mut changed = library.clone();
        changed.as_object_mut().unwrap().remove(*field);
        assert!(std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err());
    }
    let mut extra = library.clone();
    extra["fallback"] = serde_json::json!(true);
    assert!(std::panic::catch_unwind(|| check_library_sources(&root, &extra)).is_err());
    for field in [
        "kind",
        "epoch",
        "policy",
        "original_descriptor_sha256",
        "migration_sha256",
        "predecessor_descriptor_sha256",
        "predecessor_bridge_sha256",
    ] {
        let mut changed = library.clone();
        changed[field] = serde_json::json!("substitution");
        assert!(std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err());
    }
    for (field, files) in [
        ("replaced_source_hashes", REPLACED_FILES),
        ("library_source_hashes", LIBRARY_FILES),
        ("qualification_source_hashes", QUALIFICATION_FILES),
        ("adapter_source_hashes", ADAPTER_FILES),
    ] {
        for name in files {
            let mut changed = library.clone();
            changed[field].as_object_mut().unwrap().remove(*name);
            assert!(std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err());
            let mut changed = library.clone();
            if field == "replaced_source_hashes" {
                changed[field][name]["current_sha256"] = serde_json::json!("substitution");
            } else {
                changed[field][name] = serde_json::json!("substitution");
            }
            assert!(std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err());
            if field == "replaced_source_hashes" {
                for identity in ["predecessor_sha256", "current_sha256"] {
                    let mut changed = library.clone();
                    changed[field][name]
                        .as_object_mut()
                        .unwrap()
                        .remove(identity);
                    assert!(
                        std::panic::catch_unwind(|| check_library_sources(&root, &changed))
                            .is_err()
                    );
                }
                let mut changed = library.clone();
                changed[field][name]["predecessor_sha256"] = serde_json::json!("substitution");
                assert!(
                    std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err()
                );
                let mut changed = library.clone();
                changed[field][name]["extra"] = serde_json::json!("reseal");
                assert!(
                    std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err()
                );
                let mut changed = library.clone();
                changed[field][name]["current_sha256"] =
                    changed[field][name]["predecessor_sha256"].clone();
                assert!(
                    std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err()
                );
            }
        }
        let mut changed = library.clone();
        changed[field]["unexpected/source"] = serde_json::json!("fallback");
        assert!(std::panic::catch_unwind(|| check_library_sources(&root, &changed)).is_err());
    }
}

#[test]
fn repin_identity_and_report_binding_mutations_are_rejected() {
    let library: serde_json::Value = serde_json::from_str(LIBRARY).unwrap();
    let repin: serde_json::Value = serde_json::from_str(REPIN).unwrap();
    check_repin_identity(&repin, &library);
    for field in REPIN_FIELDS {
        let mut changed = repin.clone();
        changed.as_object_mut().unwrap().remove(*field);
        assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
    }
    let mut extra = repin.clone();
    extra["fallback"] = serde_json::json!(true);
    assert!(std::panic::catch_unwind(|| check_repin_identity(&extra, &library)).is_err());
    for field in [
        "kind",
        "epoch",
        "policy",
        "original_descriptor_sha256",
        "migration_sha256",
        "predecessor_descriptor_sha256",
        "predecessor_bridge_sha256",
    ] {
        let mut changed = repin.clone();
        changed[field] = serde_json::json!("substitution");
        assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
    }
    // Substituting the frozen library descriptor for the repin descriptor, and
    // resealing a replacement as a non-delta, are both rejected.
    assert!(std::panic::catch_unwind(|| check_repin_identity(&library, &library)).is_err());
    for name in REPIN_FILES {
        for identity in ["predecessor_sha256", "current_sha256"] {
            let mut changed = repin.clone();
            changed["replaced_source_hashes"][name]
                .as_object_mut()
                .unwrap()
                .remove(identity);
            assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
        }
        let mut changed = repin.clone();
        changed["replaced_source_hashes"][name]["current_sha256"] =
            changed["replaced_source_hashes"][name]["predecessor_sha256"].clone();
        assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
        let mut changed = repin.clone();
        changed["replaced_source_hashes"][name]["extra"] = serde_json::json!("reseal");
        assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
    }
    let mut changed = repin.clone();
    changed["replaced_source_hashes"]["unexpected/source"] = serde_json::json!("fallback");
    assert!(std::panic::catch_unwind(|| check_repin_identity(&changed, &library)).is_err());
}

#[test]
fn repin_report_bodies_must_bind_to_the_repin_descriptor() {
    // The defect this guards: a report whose header names one descriptor while
    // its retained producer envelopes still describe the previous one.
    let library: serde_json::Value = serde_json::from_str(LIBRARY).unwrap();
    let repin: serde_json::Value = serde_json::from_str(REPIN).unwrap();
    let report: serde_json::Value = serde_json::from_str(REPIN_BRIDGE).unwrap();
    for index in 0..2 {
        for field in ["descriptor_sha256", "kind", "replaced_source_hashes"] {
            let mut changed = report.clone();
            changed["producers"][index][field] = serde_json::json!("stale");
            let changed = serde_json::to_string(&changed).unwrap();
            assert!(
                std::panic::catch_unwind(|| check_repin_report(&changed, &repin)).is_err(),
                "stale producers[{index}].{field} must be rejected"
            );
        }
        let mut changed = report.clone();
        changed["producers"][index]["source_hashes"][REPIN_FILES[0]] =
            repin["replaced_source_hashes"][REPIN_FILES[0]]["predecessor_sha256"].clone();
        let changed = serde_json::to_string(&changed).unwrap();
        assert!(std::panic::catch_unwind(|| check_repin_report(&changed, &repin)).is_err());
    }
    let mut shared = report.clone();
    shared["producers"][1]["process"]["run_id"] =
        shared["producers"][0]["process"]["run_id"].clone();
    let shared = serde_json::to_string(&shared).unwrap();
    assert!(std::panic::catch_unwind(|| check_repin_report(&shared, &repin)).is_err());
    let mut single = report;
    single["producers"] = serde_json::json!([]);
    let single = serde_json::to_string(&single).unwrap();
    assert!(std::panic::catch_unwind(|| check_repin_report(&single, &repin)).is_err());
    let _ = library;
}

#[test]
fn loaded_map_matches_qualified_runtime_checkpoint() {
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let rom = local.join("Tenchi Souzou (Japan).sfc");
    let save = local.join("saves/Terranigma.srm");
    if !rom.try_exists().expect("inspect local ROM path")
        || !save.try_exists().expect("inspect local SRAM path")
    {
        eprintln!("skipping: local Japanese ROM or qualified SRAM not present");
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_map-inspector"))
        .arg("verify")
        .arg(rom)
        .arg(save)
        .output()
        .expect("spawn isolated map capture");
    assert!(
        output.status.success(),
        "capture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("capture manifest");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["revision"], "japan");
    // The archived complete manifest (including every cell and non-pixel surface
    // hash) is invariant across epochs, not just the selected assertions below.
    let archive: serde_json::Value = serde_json::from_str(ARCHIVE).unwrap();
    let mut nonpixels = manifest.clone();
    for cp in nonpixels["checkpoints"].as_array_mut().unwrap() {
        cp.as_object_mut().unwrap().remove("rgb_sha256");
    }
    assert_eq!(
        sha256(&serde_json::to_vec(&nonpixels).unwrap()),
        archive["nonpixel_manifest_sha256"].as_str().unwrap()
    );
    let checkpoints = manifest["checkpoints"].as_array().unwrap();
    assert_eq!(checkpoints.len(), 2);
    for (index, cp) in checkpoints.iter().enumerate() {
        assert_eq!(cp["map_id"], 0x0128);
        assert_eq!(cp["width"], 80);
        assert_eq!(cp["height"], 32);
        assert_eq!(cp["cells"].as_array().unwrap().len(), 2560);
        assert_eq!(cp["cells"][364], 0x0E11);
        assert_eq!(cp["cells"][448], 0x8007);
        assert_eq!(cp["cells"][608], 5);
        assert_eq!(cp["cells"][2559], 0x1C39);
        assert_eq!(
            cp["layer_sha256"],
            "c3c7af3a0ef3c6c53e641b058ccaccad8a9b5ea42dca79c28e41c41ecaa450c5"
        );
        assert_eq!(cp["frame"], if index == 0 { 1601 } else { 1841 });
        assert_eq!(
            cp["camera"],
            if index == 0 {
                serde_json::json!([648, 0])
            } else {
                serde_json::json!([706, 16])
            }
        );
        assert_eq!(
            cp["player"],
            if index == 0 {
                serde_json::json!([776, 112])
            } else {
                serde_json::json!([834, 128])
            }
        );
        assert_eq!(
            cp["rgb_sha256"],
            if index == 0 {
                "78c20d6a5dca2a006815c13f6577b33bcdc1ddb788ddad9949bdf889eaa7b0ea"
            } else {
                // Reviewed slot3/no-save epoch renewal; see tools/map-inspector-qualification.
                "3833dbdf403939dc5836dca4a49b49e36424360597e5acfc6eddb714372da432"
            }
        );
    }
}

// ---- Lockfile projection: the pin must stay strict where it matters. ----

const SAMPLE_LOCK: &str = r#"
[[package]]
name = "map-inspector"
version = "0.1.0"
dependencies = [
 "oracle",
]

[[package]]
name = "oracle"
version = "0.1.0"
source = "registry+x"
checksum = "abc"
dependencies = [
 "cc",
]

[[package]]
name = "cc"
version = "1.1.0"
source = "registry+x"
checksum = "def"

[[package]]
name = "unrelated"
version = "9.9.9"
source = "registry+x"
checksum = "999"
"#;

#[test]
fn the_lockfile_projection_reaches_the_whole_closure() {
    let projected = lockfile_subtree(SAMPLE_LOCK, "map-inspector");
    for name in ["map-inspector", "oracle", "cc"] {
        assert!(projected.contains(name), "{name} must be in the closure");
    }
    assert!(
        !projected.contains("unrelated"),
        "a package the producer does not depend on must not be pinned"
    );
}

#[test]
fn the_lockfile_projection_ignores_packages_outside_the_closure() {
    // The whole point: adding a workspace member the producer does not depend
    // on must not trip the gate.
    let added = format!(
        "{SAMPLE_LOCK}\n[[package]]\nname = \"newcomer\"\nversion = \"0.1.0\"\ndependencies = [\n \"cc\",\n]\n"
    );
    assert_eq!(
        lockfile_subtree(SAMPLE_LOCK, "map-inspector"),
        lockfile_subtree(&added, "map-inspector")
    );
}

#[test]
fn the_lockfile_projection_catches_every_change_inside_the_closure() {
    let baseline = lockfile_subtree(SAMPLE_LOCK, "map-inspector");
    // A version bump, a source swap, a checksum change and a dropped edge are
    // each inside the closure, and each must change the projection.
    for (from, to) in [
        ("version = \"1.1.0\"", "version = \"1.2.0\""),
        (
            "source = \"registry+x\"\nchecksum = \"def\"",
            "source = \"git+y\"\nchecksum = \"def\"",
        ),
        ("checksum = \"def\"", "checksum = \"deadbeef\""),
        ("dependencies = [\n \"cc\",\n]", "dependencies = [\n]"),
    ] {
        let mutated = SAMPLE_LOCK.replacen(from, to, 1);
        assert_ne!(mutated, SAMPLE_LOCK, "mutation {from:?} must apply");
        assert_ne!(
            lockfile_subtree(&mutated, "map-inspector"),
            baseline,
            "mutation {from:?} must change the projection"
        );
    }
}

#[test]
fn an_ambiguous_dependency_reference_widens_the_closure() {
    // Two versions of one name, referenced without a version. Taking both is
    // conservative; taking one could drop a package that reaches the producer.
    let lock = r#"
[[package]]
name = "map-inspector"
version = "0.1.0"
dependencies = [
 "twice",
]

[[package]]
name = "twice"
version = "1.0.0"
source = "registry+x"
checksum = "one"

[[package]]
name = "twice"
version = "2.0.0"
source = "registry+x"
checksum = "two"
"#;
    let projected = lockfile_subtree(lock, "map-inspector");
    assert!(projected.contains("one") && projected.contains("two"));
}

const SAMPLE_MANIFEST: &str = r#"[workspace]
resolver = "2"
members = ["crates/rom", "crates/oracle"]

[workspace.package]
version = "0.1.0"

[workspace.lints.clippy]
pedantic = "warn"
"#;

#[test]
fn the_manifest_projection_ignores_only_the_member_list() {
    let baseline = manifest_without_members(SAMPLE_MANIFEST);
    // Adding a member is invisible: the producer is built with -p.
    let added = SAMPLE_MANIFEST.replace(
        r#"members = ["crates/rom", "crates/oracle"]"#,
        r#"members = ["crates/rom", "crates/oracle", "crates/newcomer"]"#,
    );
    assert_ne!(added, SAMPLE_MANIFEST);
    assert_eq!(manifest_without_members(&added), baseline);

    // A multi-line member list is elided the same way.
    let multiline = SAMPLE_MANIFEST.replace(
        r#"members = ["crates/rom", "crates/oracle"]"#,
        "members = [\n  \"crates/rom\",\n  \"crates/oracle\",\n]",
    );
    assert_eq!(manifest_without_members(&multiline), baseline);
}

#[test]
fn the_manifest_projection_catches_everything_else() {
    let baseline = manifest_without_members(SAMPLE_MANIFEST);
    for (from, to) in [
        ("resolver = \"2\"", "resolver = \"1\""),
        ("version = \"0.1.0\"", "version = \"0.2.0\""),
        ("pedantic = \"warn\"", "pedantic = \"allow\""),
        ("[workspace.package]", "[profile.release]"),
    ] {
        let mutated = SAMPLE_MANIFEST.replacen(from, to, 1);
        assert_ne!(mutated, SAMPLE_MANIFEST, "mutation {from:?} must apply");
        assert_ne!(
            manifest_without_members(&mutated),
            baseline,
            "mutation {from:?} must change the projection"
        );
    }
    // An added profile table is caught too.
    let profiled = format!("{SAMPLE_MANIFEST}\n[profile.release]\nlto = true\n");
    assert_ne!(manifest_without_members(&profiled), baseline);
}

#[test]
fn a_duplicate_name_and_version_at_another_source_is_not_merged() {
    // Keying the closure on (name, version) alone merges a patched git fork
    // that kept its version number with the registry package it replaces. The
    // second one's edges then never get walked, so a whole subtree -- and its
    // checksums -- drop out of the pin silently.
    let lock = r#"
[[package]]
name = "map-inspector"
version = "0.1.0"
dependencies = [
 "foo 1.0.0 (registry+x)",
 "bar",
]

[[package]]
name = "bar"
version = "1.0.0"
source = "registry+x"
checksum = "bar-sum"
dependencies = [
 "foo 1.0.0 (git+fork)",
]

[[package]]
name = "foo"
version = "1.0.0"
source = "registry+x"
checksum = "registry-sum"

[[package]]
name = "foo"
version = "1.0.0"
source = "git+fork"
checksum = "fork-sum"
dependencies = [
 "hidden",
]

[[package]]
name = "hidden"
version = "6.6.6"
source = "registry+x"
checksum = "hidden-sum"
"#;
    let projected = lockfile_subtree(lock, "map-inspector");
    assert!(projected.contains("registry-sum"), "registry foo");
    assert!(projected.contains("fork-sum"), "the forked foo");
    assert!(
        projected.contains("hidden-sum"),
        "a package reachable only through the fork must be in the closure"
    );
    // And each of those checksums must actually be load-bearing.
    for sum in ["registry-sum", "fork-sum", "hidden-sum"] {
        let mutated = lock.replacen(sum, "tampered", 1);
        assert_ne!(
            lockfile_subtree(&mutated, "map-inspector"),
            projected,
            "tampering with {sum} must change the projection"
        );
    }
}

#[test]
fn the_member_elision_cannot_run_past_its_own_line() {
    // A trailing comment stops the members line ending in `]`. A heuristic that
    // elides until the next `]`-terminated line would swallow everything
    // between, hiding injected workspace keys inside the elided region.
    let baseline = manifest_without_members(SAMPLE_MANIFEST);
    let smuggled = SAMPLE_MANIFEST.replace(
        r#"members = ["crates/rom", "crates/oracle"]"#,
        "members = [\"crates/rom\", \"crates/oracle\"] #\ndependencies = { serde = \"1\" }\n[patch.crates-io]",
    );
    assert_ne!(smuggled, SAMPLE_MANIFEST);
    assert_ne!(
        manifest_without_members(&smuggled),
        baseline,
        "keys smuggled after the members line must remain visible"
    );
    assert!(
        manifest_without_members(&smuggled).contains("patch.crates-io"),
        "the injected table must survive the projection"
    );
}

#[test]
fn only_the_workspace_tables_members_key_is_elided() {
    // A `members`-prefixed key in another table is a different key.
    let elsewhere =
        format!("{SAMPLE_MANIFEST}\n[workspace.metadata.x]\nmembers_are_cool = \"yes\"\n");
    assert!(
        manifest_without_members(&elsewhere).contains("members_are_cool"),
        "a key outside [workspace] must not be elided"
    );
    // default-members is a distinct key and stays pinned.
    let defaults = SAMPLE_MANIFEST.replace(
        r#"members = ["crates/rom", "crates/oracle"]"#,
        "members = [\"crates/rom\"]\ndefault-members = [\"crates/rom\"]",
    );
    assert!(manifest_without_members(&defaults).contains("default-members"));
}

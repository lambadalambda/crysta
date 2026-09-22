//! Bounded native play-session JSONL diagnostics, independent of game assets.
//!
//! Four owned session slots retain at most two 8 MiB segments each (64 MiB total,
//! plus tiny ownership markers). Rotation discards history: this is a diagnostic
//! tail, not a complete replay. Each segment repeats its session header and has a
//! monotonic `segment` number; read surviving segments in that order.
//!
//! Use one active logger per caller-controlled root. Unknown entries and symlinks
//! are not reclaimed; occupied/unowned slots can reduce retention or prevent
//! logging. This is not a security boundary against concurrent filesystem edits.
//! Events and metadata are caller-supplied: do not include ROM bytes or secrets.

use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const OWNER: &str = "crysta diagnostics v1\n";
const MARKER: &str = ".owner";
const SEGMENTS: [&str; 2] = ["events-0.jsonl", "events-1.jsonl"];

#[derive(Clone, Copy)]
struct Limits {
    segment_bytes: usize,
    flush_records: usize,
    flush_interval: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            segment_bytes: 8 * 1024 * 1024,
            flush_records: 60,
            flush_interval: Duration::from_secs(1),
        }
    }
}

/// A synchronous buffered diagnostic trace. Disable logging after an I/O error.
pub struct SessionLog {
    path: PathBuf,
    writer: BufWriter<File>,
    header: Value,
    segment: u64,
    bytes: usize,
    header_bytes: usize,
    pending: usize,
    last_flush: Instant,
    limits: Limits,
}

impl SessionLog {
    /// Start a session, flushing its generated schema-1 header immediately.
    ///
    /// # Errors
    /// Returns filesystem errors, or an error if no safe slot is available or
    /// metadata cannot fit in one segment. Never reclaims unowned directories.
    pub fn start(root: &Path, metadata: Value) -> io::Result<Self> {
        Self::with_limits(root, metadata, Limits::default())
    }

    fn with_limits(root: &Path, metadata: Value, limits: Limits) -> io::Result<Self> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let mut header = json!({
            "kind": "session", "schema": 1,
            "session": format!("{stamp}-{}", std::process::id()),
            "segment": 0,
        });
        header["metadata"] = metadata;
        let encoded = json_line(&header)?;
        ensure_fits(encoded.len(), limits.segment_bytes)?;
        fs::create_dir_all(root)?;
        if !fs::symlink_metadata(root)?.file_type().is_dir() {
            return Err(io::Error::other(
                "log root must be a directory, not a symlink",
            ));
        }
        let directory = reserve_slot(root)?;
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(MARKER))?;
        write!(marker, "{OWNER}{stamp}")?;
        let path = directory.join(SEGMENTS[0]);
        let mut writer = fresh_writer(&path)?;
        writer.write_all(&encoded)?;
        writer.flush()?;
        Ok(Self {
            path,
            writer,
            header,
            segment: 0,
            bytes: encoded.len(),
            header_bytes: encoded.len(),
            pending: 0,
            last_flush: Instant::now(),
            limits,
        })
    }

    /// Current JSONL segment path; may change after a record triggers rotation.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append an event unchanged. Urgent records flush all preceding records too.
    ///
    /// Normal writes flush every 60 records or on the first write at least one
    /// second after the last flush. There is no idle timer, worker or `fsync`.
    ///
    /// # Errors
    /// Returns serialization/filesystem errors. An entry too large to fit beside
    /// its header is rejected before changing files. On other I/O errors the tail
    /// may be incomplete; the caller should disable logging rather than retry.
    pub fn record(&mut self, event: &Value, urgent: bool) -> io::Result<()> {
        let encoded = json_line(event)?;
        ensure_fits(encoded.len(), self.limits.segment_bytes - self.header_bytes)?;
        if encoded.len() > self.limits.segment_bytes - self.bytes {
            self.rotate(encoded.len())?;
        }
        self.writer.write_all(&encoded)?;
        self.bytes += encoded.len();
        self.pending += 1;
        if urgent
            || self.pending >= self.limits.flush_records
            || self.last_flush.elapsed() >= self.limits.flush_interval
        {
            self.flush()?;
        }
        Ok(())
    }

    /// Flush pending records to the operating system, without forcing disk sync.
    ///
    /// # Errors
    /// Returns the underlying writer's I/O error.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.pending = 0;
        self.last_flush = Instant::now();
        Ok(())
    }

    fn rotate(&mut self, entry_bytes: usize) -> io::Result<()> {
        let segment = self
            .segment
            .checked_add(1)
            .ok_or_else(|| io::Error::other("segment counter exhausted"))?;
        let mut header = self.header.clone();
        header["segment"] = segment.into();
        let encoded = json_line(&header)?;
        ensure_fits(encoded.len(), self.limits.segment_bytes)?;
        ensure_fits(entry_bytes, self.limits.segment_bytes - encoded.len())?;
        self.flush()?;
        let path = self.path.with_file_name(if segment % 2 == 0 {
            SEGMENTS[0]
        } else {
            SEGMENTS[1]
        });
        let mut writer = fresh_writer(&path)?;
        writer.write_all(&encoded)?;
        writer.flush()?;
        self.writer = writer;
        self.path = path;
        self.header = header;
        self.segment = segment;
        self.bytes = encoded.len();
        self.header_bytes = encoded.len();
        Ok(())
    }
}

impl Drop for SessionLog {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn json_line(value: &Value) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn ensure_fits(bytes: usize, available: usize) -> io::Result<()> {
    if bytes > available {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "diagnostic entry exceeds segment limit",
        ));
    }
    Ok(())
}

// Unlink rather than truncate: even an existing hard link must not change its target.
fn remove_regular(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => fs::remove_file(path),
        Ok(_) => Err(io::Error::other(
            "refusing to replace a non-regular log file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn fresh_writer(path: &Path) -> io::Result<BufWriter<File>> {
    remove_regular(path)?;
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    Ok(BufWriter::with_capacity(16 * 1024, file))
}

// Only reclaim directories with our exact marker and exclusively known regular files.
fn owned_stamp(directory: &Path) -> io::Result<Option<u128>> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if !entry.file_type()?.is_file()
            || !(name == MARKER || SEGMENTS.iter().any(|segment| name == *segment))
        {
            return Ok(None);
        }
    }
    let marker = match File::open(directory.join(MARKER)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut contents = String::new();
    if marker.take(128).read_to_string(&mut contents).is_err() {
        return Ok(None);
    }
    Ok(contents
        .strip_prefix(OWNER)
        .and_then(|stamp| stamp.parse().ok()))
}

fn reserve_slot(root: &Path) -> io::Result<PathBuf> {
    let mut oldest: Option<(u128, PathBuf)> = None;
    for slot in 0..4 {
        let directory = root.join(format!("crysta-session-{slot}"));
        match fs::symlink_metadata(&directory) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&directory)?;
                return Ok(directory);
            }
            Err(error) => return Err(error),
            Ok(metadata) if metadata.file_type().is_dir() => {
                if let Some(stamp) = owned_stamp(&directory)? {
                    if oldest
                        .as_ref()
                        .is_none_or(|(previous, _)| stamp < *previous)
                    {
                        oldest = Some((stamp, directory));
                    }
                }
            }
            Ok(_) => {} // User files and symlink slots are never followed or removed.
        }
    }
    let (_, directory) =
        oldest.ok_or_else(|| io::Error::other("no safe diagnostic session slot"))?;
    for name in SEGMENTS.into_iter().chain([MARKER]) {
        remove_regular(&directory.join(name))?;
    }
    Ok(directory)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "crysta-diagnostics-test-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn limits() -> Limits {
        Limits {
            segment_bytes: 512,
            flush_records: 3,
            flush_interval: Duration::from_secs(1),
        }
    }

    fn lines(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn segments(root: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for slot in 0..4 {
            for segment in 0..2 {
                let path = root
                    .join(format!("crysta-session-{slot}"))
                    .join(format!("events-{segment}.jsonl"));
                if path.is_file() {
                    paths.push(path);
                }
            }
        }
        paths
    }

    #[test]
    fn header_and_urgent_records_are_immediately_visible_and_ordered() {
        let temp = TempDir::new();
        let mut log = SessionLog::with_limits(
            &temp.0,
            json!({"build": "test"}),
            Limits {
                flush_interval: Duration::from_hours(1),
                ..Limits::default()
            },
        )
        .unwrap();
        let header = lines(log.path());
        assert_eq!(header.len(), 1);
        assert_eq!(header[0]["kind"], "session");
        assert_eq!(header[0]["schema"], 1);
        assert_eq!(header[0]["metadata"], json!({"build": "test"}));
        assert!(!header[0]["session"].as_str().unwrap().is_empty());
        let first = json!({"tick": 1, "inputs": ["right", "confirm"], "text": "newline\nquote\""});
        let second = json!({"tick": 2, "outcome": "refused"});
        log.record(&first, false).unwrap();
        assert_eq!(lines(log.path()).len(), 1);
        log.record(&second, true).unwrap();
        assert_eq!(lines(log.path())[1..], [first, second]);
    }

    #[test]
    fn count_and_elapsed_time_flush_without_a_worker() {
        let temp = TempDir::new();
        let mut log = SessionLog::with_limits(&temp.0, json!({}), limits()).unwrap();
        for tick in 0..3 {
            log.record(&json!({"tick": tick}), false).unwrap();
        }
        assert_eq!(lines(log.path()).len(), 4);
        log.last_flush = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
        log.record(&json!({"tick": 3}), false).unwrap();
        assert_eq!(lines(log.path()).len(), 5);
    }

    #[test]
    fn explicit_flush_and_drop_preserve_pending_records() {
        let temp = TempDir::new();
        let mut log = SessionLog::start(&temp.0, json!({})).unwrap();
        let path = log.path().to_owned();
        log.record(&json!("explicit"), false).unwrap();
        log.flush().unwrap();
        assert_eq!(lines(&path).len(), 2);
        log.record(&json!("drop"), false).unwrap();
        drop(log);
        assert_eq!(lines(&path)[2], "drop");
    }

    #[test]
    fn rotation_and_retention_keep_only_a_bounded_recent_tail() {
        let temp = TempDir::new();
        for run in 0..8 {
            let mut log = SessionLog::with_limits(&temp.0, json!({"run": run}), limits()).unwrap();
            for tick in 0..10 {
                log.record(&json!({"tick": tick, "padding": "x".repeat(230)}), false)
                    .unwrap();
            }
        }
        let paths = segments(&temp.0);
        assert_eq!(paths.len(), 8);
        let mut runs = std::collections::BTreeSet::new();
        for path in paths {
            assert!(fs::metadata(&path).unwrap().len() <= 512);
            let records = lines(&path);
            runs.insert(records[0]["metadata"]["run"].as_u64().unwrap());
            let segment = records[0]["segment"].as_u64().unwrap();
            assert!(segment >= 8, "only the last two segments survive");
            assert_eq!(records[1]["tick"], segment);
        }
        assert_eq!(runs, [4, 5, 6, 7].into_iter().collect());
    }

    #[test]
    fn oversized_records_and_headers_fail_without_growing_files() {
        let temp = TempDir::new();
        let mut log = SessionLog::with_limits(&temp.0, json!({}), limits()).unwrap();
        let before = fs::read(log.path()).unwrap();
        let huge = json!("x".repeat(1024));
        assert!(log.record(&huge, true).is_err());
        assert_eq!(fs::read(log.path()).unwrap(), before);
        assert_eq!(segments(&temp.0).len(), 1);
        assert!(SessionLog::with_limits(&temp.0, huge, limits()).is_err());
        assert_eq!(segments(&temp.0).len(), 1);
    }

    #[test]
    fn unknown_files_and_unowned_directories_are_untouched() {
        let temp = TempDir::new();
        fs::write(temp.0.join("notes.txt"), "user notes").unwrap();
        let reserved = temp.0.join("crysta-session-0");
        fs::create_dir(&reserved).unwrap();
        fs::write(reserved.join("events-0.jsonl"), "not ours").unwrap();
        let first = SessionLog::start(&temp.0, json!({})).unwrap();
        let protected = first.path().parent().unwrap().join("notes.txt");
        fs::write(&protected, "also user notes").unwrap();
        let preserved_log = first.path().to_owned();
        let before = fs::read(&preserved_log).unwrap();
        drop(first);
        for _ in 0..8 {
            drop(SessionLog::start(&temp.0, json!({})).unwrap());
        }
        assert_eq!(
            fs::read_to_string(temp.0.join("notes.txt")).unwrap(),
            "user notes"
        );
        assert_eq!(
            fs::read_to_string(reserved.join("events-0.jsonl")).unwrap(),
            "not ours"
        );
        assert_eq!(fs::read_to_string(protected).unwrap(), "also user notes");
        assert_eq!(fs::read(preserved_log).unwrap(), before);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_slots_and_segment_targets_are_untouched() {
        use std::os::unix::fs::symlink;
        let temp = TempDir::new();
        let outside = TempDir::new();
        let victim = outside.0.join("events-0.jsonl");
        fs::write(&victim, "user data").unwrap();
        let slot = temp.0.join("crysta-session-0");
        symlink(&outside.0, &slot).unwrap();
        let first = SessionLog::start(&temp.0, json!({})).unwrap();
        let path = first.path().to_owned();
        drop(first);
        fs::remove_file(&path).unwrap();
        symlink(&victim, &path).unwrap();
        for _ in 0..8 {
            drop(SessionLog::start(&temp.0, json!({})).unwrap());
        }
        assert!(fs::symlink_metadata(slot).unwrap().file_type().is_symlink());
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(victim).unwrap(), "user data");
    }

    #[test]
    fn unavailable_root_or_slots_return_errors() {
        let temp = TempDir::new();
        let file = temp.0.join("file");
        fs::write(&file, "user data").unwrap();
        assert!(SessionLog::start(&file, json!({})).is_err());
        for slot in 0..4 {
            fs::create_dir(temp.0.join(format!("crysta-session-{slot}"))).unwrap();
        }
        assert!(SessionLog::start(&temp.0, json!({})).is_err());
    }
}

//! Can this binary account for the source it is about to measure with?
//!
//! THE HAZARD THIS EXISTS FOR
//! -------------------------
//! `scripts/falsify.sh` breaks a guard, watches the named test fail, restores the
//! source and PROVES the restore with a hash. What it did not do was rebuild. Its
//! own `cargo test --release` builds the BINARY target as well as the tests, so
//! after every falsification `target/release/bubble-watch` was a binary built from
//! the MUTATED source while the working tree was clean.
//!
//! `scripts/daily-refresh.sh` rebuilt only when the binary was MISSING, so the next
//! 13:32 timer run executed that binary and `site` RECORDED its numbers into
//! `data/history/runs.jsonl`. The falsifications in question rewrite an indicator's
//! output text, so the mutation would have shipped into the archive and the
//! dashboard, with a clean tree and `--verify` passing.
//!
//! MEASURED, TWICE. On 2026-10-17 a falsification of the `report` module was run
//! and the binary left behind printed, offline from the same cache, "plus 5 things
//! it cannot measure at all" where the committed source prints 2 — the mutant's
//! text, from a clean tree. And a binary with mtime 17:39 emitted the mutant text
//! of an EARLIER falsification. Both are recorded in the commit that added this
//! guard.
//!
//! WHAT THE GUARD IS
//! -----------------
//! `build.rs` bakes the fingerprint of the source tree (see
//! `crate::build_fingerprint`) into the binary at compile time. At run time this
//! module recomputes the fingerprint of that same tree and compares. Three verdicts:
//!
//!   * `Verified` — the tree is byte-identical to the one this binary was compiled
//!     from. Anything the run produces comes from the code that is there now.
//!   * `Mismatch` — the tree has changed since the build. This is exactly the
//!     leaked-falsification state: mutant binary, restored source.
//!   * `Unknown` — the tree could not be read, or no fingerprint was baked in.
//!
//! RECORDING REQUIRES `Verified`. A run that cannot account for its own code must
//! not enter the measurement archive; that rule lives in `crate::history::decide_record`
//! next to the other recording rule (an offline run may not record either) so no
//! command can forget it. `Unknown` is refused too: "we could not tell" is not a
//! reason to file a number in the archive, and the failure direction of guessing
//! wrong here is a falsified measurement that looks exactly like a real one.
//!
//! THIS IS NOT A SECURITY BOUNDARY. See `crate::build_fingerprint`: the hash is
//! FNV-1a/64 and detects accidental divergence, not a forged build.

use std::path::Path;

/// The fingerprint of the tree this binary was compiled from, baked in by
/// `build.rs`. EMPTY means the build could not fingerprint its tree.
pub const EMBEDDED: &str = env!("BUBBLE_WATCH_SOURCE_DIGEST");

/// How many files that fingerprint covered at build time. Used only for reporting.
pub const EMBEDDED_FILES: &str = env!("BUBBLE_WATCH_SOURCE_FILES");

/// The tree the binary was built in, baked in by rustc from `CARGO_MANIFEST_DIR`.
///
/// Deliberately the BUILD-time path and not the current working directory: the
/// question is whether the binary still matches the source it came out of, and a
/// checkout that has moved or been replaced answers that question with a mismatch
/// rather than with a false pass.
pub const SOURCE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The verdict on the running binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The tree is exactly what this binary was built from.
    Verified { digest: String, files: usize },
    /// The tree has changed since this binary was built.
    Mismatch {
        embedded: String,
        on_disk: String,
        files: usize,
    },
    /// The comparison could not be made.
    Unknown { embedded: String, reason: String },
}

impl Status {
    pub fn is_verified(&self) -> bool {
        matches!(self, Status::Verified { .. })
    }

    /// One word for a terminal line or a shell's `grep`.
    pub fn verdict_word(&self) -> &'static str {
        match self {
            Status::Verified { .. } => "VERIFIED",
            Status::Mismatch { .. } => "MISMATCH",
            Status::Unknown { .. } => "UNKNOWN",
        }
    }

    /// The fingerprint baked into the binary, or an empty string if none was.
    pub fn baked(&self) -> &str {
        match self {
            Status::Verified { digest, .. } => digest,
            Status::Mismatch { embedded, .. } => embedded,
            Status::Unknown { embedded, .. } => embedded,
        }
    }

    /// The fingerprint of the tree as it is now, when it could be computed.
    pub fn on_disk(&self) -> Option<&str> {
        match self {
            Status::Verified { digest, .. } => Some(digest),
            Status::Mismatch { on_disk, .. } => Some(on_disk),
            Status::Unknown { .. } => None,
        }
    }

    /// A single line stating the verdict and its evidence.
    ///
    /// Capitalised, because it is interpolated as a WHOLE SENTENCE into the refusal
    /// text: joining a lowercase fragment to "…arises from an unbuilt edit." was the
    /// first version and it read as one run-on sentence.
    pub fn summary(&self) -> String {
        match self {
            Status::Verified { digest, files } => format!(
                "The tree is byte-identical to the one this binary was built from \
                 (fingerprint {}, {} file(s)).",
                digest, files
            ),
            Status::Mismatch {
                embedded,
                on_disk,
                files,
            } => format!(
                "The tree is NOT what this binary was built from: baked fingerprint {}, tree \
                 fingerprint now {} ({} file(s)).",
                embedded, on_disk, files
            ),
            Status::Unknown { embedded, reason } => {
                if embedded.is_empty() {
                    format!(
                        "No source fingerprint was baked into this binary at build time ({}).",
                        reason
                    )
                } else {
                    format!(
                        "The source tree could not be fingerprinted to compare against the baked \
                         value {} ({}).",
                        embedded, reason
                    )
                }
            }
        }
    }

    /// The refusal text for the archive, or `None` when recording is permitted.
    ///
    /// This is a WARNING STRING, not an error code: it is pushed onto the report so
    /// it reaches the HTML, the JSON and the terminal (the terminal is the part
    /// that was missing for the offline rule and cost a day of diagnosis).
    /// `main` additionally exits non-zero when a requested record did not happen,
    /// so an unattended run cannot fail quietly.
    pub fn refusal(&self) -> Option<String> {
        match self {
            Status::Verified { .. } => None,
            Status::Mismatch { .. } => Some(format!(
                "NOT RECORDED: this binary was not built from the source tree at {}, so the \
                 numbers it produced may come from code that is no longer there. That is the \
                 signature of a LEAKED FALSIFICATION — scripts/falsify.sh restores the source \
                 after breaking a guard, and its `cargo test` leaves the BINARY built from the \
                 mutated source — and the same state arises from an unbuilt edit. {} Nothing \
                 was appended to the archive. Rebuild (`cargo build --release`) and re-run.",
                SOURCE_ROOT,
                self.summary()
            )),
            Status::Unknown { .. } => Some(format!(
                "NOT RECORDED: this run cannot show that it was built from the source tree at \
                 {} — {}. A run that cannot account for its own code must not enter the \
                 measurement archive, because a number filed there is indistinguishable from a \
                 real measurement. Nothing was appended to the archive. Rebuild \
                 (`cargo build --release`) from a complete checkout and re-run.",
                SOURCE_ROOT,
                self.summary()
            )),
        }
    }
}

/// Compare an embedded fingerprint with the tree at `root`.
///
/// The comparison is PURE and separated from the walking so it can be tested and
/// falsified by name, with synthetic fingerprints, on any machine.
pub fn verdict(embedded: &str, on_disk: &str, files: usize) -> Status {
    if embedded.is_empty() {
        return Status::Unknown {
            embedded: String::new(),
            reason: "BUBBLE_WATCH_SOURCE_DIGEST was empty at compile time".to_string(),
        };
    }
    if embedded == on_disk {
        Status::Verified {
            digest: on_disk.to_string(),
            files,
        }
    } else {
        Status::Mismatch {
            embedded: embedded.to_string(),
            on_disk: on_disk.to_string(),
            files,
        }
    }
}

/// Fingerprint the tree at `root` and compare it with `embedded`.
pub fn check_in(root: &Path, embedded: &str) -> Status {
    match crate::build_fingerprint::fingerprint_with_count(root) {
        Ok((on_disk, files)) => verdict(embedded, &on_disk, files),
        Err(reason) => Status::Unknown {
            embedded: embedded.to_string(),
            reason,
        },
    }
}

/// Check the RUNNING binary against the tree it was built in.
pub fn check() -> Status {
    check_in(Path::new(SOURCE_ROOT), EMBEDDED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_that_does_not_match_the_tree_is_not_verified() {
        // THE GUARD. This is the leaked-falsification state: the binary was built
        // from other source (a mutant) and the tree has been restored. It must NOT
        // read as verified, and it must say so loudly enough to be found in a
        // journal — the refusal has to name both fingerprints.
        let s = verdict("aaaaaaaaaaaaaaaa", "bbbbbbbbbbbbbbbb", 41);
        assert_eq!(s.verdict_word(), "MISMATCH");
        assert!(!s.is_verified());
        let r = s.refusal().expect("a mismatch must refuse to record");
        assert!(
            r.starts_with("NOT RECORDED"),
            "the refusal must be greppable as a refusal: {}",
            r
        );
        assert!(
            r.contains("aaaaaaaaaaaaaaaa"),
            "must name the baked value: {}",
            r
        );
        assert!(
            r.contains("bbbbbbbbbbbbbbbb"),
            "must name what the tree is now: {}",
            r
        );
        assert!(
            r.to_lowercase().contains("falsification"),
            "must name the likely cause rather than only the fact: {}",
            r
        );
    }

    #[test]
    fn a_matching_fingerprint_is_verified_and_refuses_nothing() {
        // The complement, so the guard cannot be satisfied by refusing everything:
        // a binary built from the tree beside it must be allowed to record.
        let s = verdict("0123456789abcdef", "0123456789abcdef", 41);
        assert!(s.is_verified());
        assert_eq!(s.verdict_word(), "VERIFIED");
        assert!(s.refusal().is_none(), "a verified binary must not refuse");
    }

    #[test]
    fn a_binary_with_no_baked_fingerprint_is_unknown_not_verified() {
        // A build that could not fingerprint its tree must not read as a pass. The
        // reassuring direction to fail in is the dangerous one.
        let s = verdict("", "0123456789abcdef", 41);
        assert_eq!(s.verdict_word(), "UNKNOWN");
        assert!(!s.is_verified());
        let r = s.refusal().expect("an unverifiable run must not record");
        assert!(r.starts_with("NOT RECORDED"));
        assert!(r.contains("empty at compile time"), "must say why: {}", r);
    }

    #[test]
    fn an_unreadable_tree_is_unknown_not_verified() {
        let s = check_in(
            Path::new("/nonexistent-bubble-watch-tree"),
            "0123456789abcdef",
        );
        assert_eq!(s.verdict_word(), "UNKNOWN");
        assert!(!s.is_verified());
        assert!(
            s.refusal().unwrap().contains("no source directory"),
            "must say what stopped it: {}",
            s.summary()
        );
    }

    #[test]
    fn the_fingerprint_notices_a_one_byte_change_and_tracks_the_file_list() {
        // The fingerprint is the whole guard, so its own properties are tested:
        // content changes, additions and removals must all move it, and the count
        // must follow the file list.
        let dir = std::env::temp_dir().join(format!("bw-fingerprint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/indicators")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn a() -> u8 { 1 }\n").unwrap();
        std::fs::write(
            dir.join("src/indicators/mod.rs"),
            "pub fn b() -> u8 { 2 }\n",
        )
        .unwrap();

        let (d1, n1) = crate::build_fingerprint::fingerprint_with_count(&dir).unwrap();
        assert_eq!(n1, 3, "Cargo.toml plus both .rs files");

        // A one-byte content change.
        std::fs::write(dir.join("src/lib.rs"), "pub fn a() -> u8 { 2 }\n").unwrap();
        let (d2, n2) = crate::build_fingerprint::fingerprint_with_count(&dir).unwrap();
        assert_ne!(d1, d2, "a content change must move the fingerprint");
        assert_eq!(n1, n2);

        // A removal.
        std::fs::remove_file(dir.join("src/indicators/mod.rs")).unwrap();
        let (d3, n3) = crate::build_fingerprint::fingerprint_with_count(&dir).unwrap();
        assert_ne!(d2, d3, "removing a file must move the fingerprint");
        assert_eq!(n3, 2);

        // An addition.
        std::fs::write(dir.join("src/extra.rs"), "").unwrap();
        let (d4, n4) = crate::build_fingerprint::fingerprint_with_count(&dir).unwrap();
        assert_ne!(
            d3, d4,
            "adding an EMPTY file must move the fingerprint — the file count is folded in \
             precisely so a new file cannot hide behind empty content"
        );
        assert_eq!(n4, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_build_recorded_a_fingerprint_and_the_two_implementations_agree_on_the_file_set() {
        // WHAT THIS ASSERTS, AND WHAT IT DELIBERATELY DOES NOT.
        //
        // It asserts the part that cannot move under a running build: a fingerprint was
        // baked in, it is a 16-hex-digit digest, and the file COUNT the build script
        // recorded is the count the RUN-TIME walker sees over the same tree. Those are
        // the properties that would break if `build.rs` stopped running, or if the two
        // callers of `build_fingerprint` walked different sets of files.
        //
        // It does NOT assert that the baked DIGEST equals the tree's digest now, and
        // that is deliberate rather than lazy. This repository is worked by more than one
        // process at a time: measured 2026-10-17, a SIBLING WORKER edited
        // `src/report/layman.rs` while a `cargo test` run was in flight, and a
        // digest-equality version of this test FAILED — while the guard itself was
        // working exactly as designed (the binary no longer matched the tree, which is
        // the state it exists to detect). A test that fails whenever a colleague saves a
        // file teaches its reader to edit the test instead of reading the failure.
        //
        // The digest comparison is a RUN-TIME property, and it is enforced where it
        // matters: `bubble-watch artifact-check` reports it, `history::decide_record`
        // refuses to record without it, and `scripts/falsify.sh` and
        // `scripts/daily-refresh.sh` both gate on it.
        assert!(
            !EMBEDDED.is_empty(),
            "build.rs did not bake a source fingerprint (BUBBLE_WATCH_SOURCE_DIGEST is empty), \
             so every run from this binary would refuse to record"
        );
        assert_eq!(
            EMBEDDED.len(),
            16,
            "the fingerprint is 16 hex digits, got {:?}",
            EMBEDDED
        );
        assert!(
            EMBEDDED.chars().all(|c| c.is_ascii_hexdigit()),
            "the fingerprint must be hex, got {:?}",
            EMBEDDED
        );
        let root = Path::new(SOURCE_ROOT);
        let files = crate::build_fingerprint::tracked_files(root)
            .expect("the tree this binary was built in must still be readable");
        assert_eq!(
            files.len().to_string(),
            EMBEDDED_FILES,
            "the build script and the run-time walker disagree on HOW MANY files make up the \
             fingerprint, so one of them is not walking src/**/*.rs + Cargo.toml + Cargo.lock"
        );
        // And the walk is deterministic: the same tree must always fingerprint the same,
        // or no comparison between a baked value and a live one could ever be stable.
        let a = crate::build_fingerprint::fingerprint(root).unwrap();
        let b = crate::build_fingerprint::fingerprint(root).unwrap();
        assert_eq!(a, b, "fingerprinting the same tree twice must agree");
    }
}

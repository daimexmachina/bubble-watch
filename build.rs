//! Bake the source tree's fingerprint into the binary at compile time.
//!
//! WHY. `scripts/falsify.sh` mutates a source file, runs `cargo test --release`
//! (which builds the BINARY target too), then restores and hash-verifies the
//! SOURCE. The restore is proved; the BINARY is not, and it is left built from the
//! mutated source. `scripts/daily-refresh.sh` builds only when the binary is
//! MISSING, so the next timer run executes that binary and records its numbers into
//! `data/history/runs.jsonl` — a falsified measurement in the archive, with a clean
//! tree and `--verify` passing.
//!
//! This script is the only place the *build* can know what it was built from, so
//! the digest goes in here and `src/build_id.rs` compares it against the tree at
//! run time.
//!
//! The walk and the hash live in `src/build_fingerprint.rs`, included verbatim from
//! there so the build-time and run-time fingerprints cannot drift apart. The file
//! is pure std; it has no crate dependencies, which is what makes it includable in
//! a build script.

// NOTE: no `use std::path::...` here. The included file imports exactly what it needs, and a
// second import of the same type in the same module is E0252. `PathBuf` below comes from it.

include!("src/build_fingerprint.rs");

fn main() {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));

    // Re-run this script when any tracked FILE changes, and when any DIRECTORY in
    // the walk changes (a directory's mtime moves when a file is added or removed).
    // Emitting only the files would miss a newly added source file: cargo would
    // compile it, and the fingerprint would not know it exists.
    for f in tracked_files(&root).unwrap_or_default() {
        println!("cargo:rerun-if-changed={}", f.display());
    }
    for d in tracked_dirs(&root).unwrap_or_default() {
        println!("cargo:rerun-if-changed={}", d.display());
    }
    // A change to this script changes what the binary claims about itself.
    println!("cargo:rerun-if-changed=build.rs");

    match fingerprint_with_count(&root) {
        Ok((digest, files)) => {
            println!("cargo:rustc-env=BUBBLE_WATCH_SOURCE_DIGEST={}", digest);
            // The count travels with the binary so a mismatch report can say how
            // much of the tree it read rather than only that it differed.
            println!("cargo:rustc-env=BUBBLE_WATCH_SOURCE_FILES={}", files);
        }
        Err(e) => {
            // NO SILENT FALLBACK. An empty digest means "this binary cannot account
            // for its source", which `build_id::Status` treats as UNKNOWN and
            // refuses to record from — deliberately not as "matches".
            println!(
                "cargo:warning=bubble-watch: the source tree could not be fingerprinted ({e}); \
                 the binary will refuse to record a measurement until it is rebuilt from a \
                 complete checkout"
            );
            println!("cargo:rustc-env=BUBBLE_WATCH_SOURCE_DIGEST=");
            println!("cargo:rustc-env=BUBBLE_WATCH_SOURCE_FILES=0");
        }
    }
}

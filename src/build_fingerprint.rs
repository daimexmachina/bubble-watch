// A content fingerprint of the source tree, computed the SAME WAY at build time
// and at run time.
//
// WHY THIS EXISTS, AND WHY IT IS CONTENT RATHER THAN MTIME
// -------------------------------------------------------
// `src/build_id.rs` refuses to let a run enter the measurement archive unless the
// running binary can show that it was built from the source tree it names. That
// needs two fingerprints of the same tree, taken at different moments: one baked
// into the binary by `build.rs`, one recomputed from disk when the run happens.
//
// The obvious alternative is an mtime comparison ("is any source file newer than
// the binary?"). It was rejected because mtime is a PROXY for content, and the
// proxy fails in both directions:
//
//   * FALSE NEGATIVE. `cp -p`, `git checkout`, `tar -x`, a restored backup or a
//     build cache can all leave a file whose CONTENT differs from the binary's
//     while its mtime is older. The hazard this guards against was MEASURED on
//     2026-10-17: `scripts/falsify.sh` restores the mutated source with `cp`, so
//     the restored file is younger than the mutant binary and mtime happens to
//     catch it — but that is a property of `cp`, not of the guard, and it is one
//     edit away from silently failing.
//   * FALSE POSITIVE. `touch` on any source file, a fresh `git clone`, or a
//     `git stash`/`git stash pop` cycle makes every source newer than a binary
//     built from byte-identical content. Refusing to record there would be a
//     guard that fires on nothing and trains its reader to ignore it.
//
// The fingerprint compares what actually decides the binary's behaviour, so a
// `touch` neither blocks a run nor hides one.
//
// WHAT IS FINGERPRINTED
// ---------------------
// `Cargo.toml`, `Cargo.lock`, and every `*.rs` file under `src/`, recursively.
// Paths are included RELATIVE to the tree root, so the digest is stable when the
// checkout moves and changes the moment a file is renamed, added or removed.
//
// NOT a security boundary. The hash is FNV-1a/64: cheap, dependency-free and
// adequate for detecting an accidental difference (a leaked falsification, an
// unbuilt edit, a stale artifact). It is not collision-resistant, and it is not
// meant to be — nothing here defends against an adversary who would rather forge a
// build than fix it.
//
// WHY THESE COMMENTS ARE `//` AND NOT `//!`. This file is included into `build.rs`
// by `include!`, which splices ITEMS and cannot carry an inner attribute: a `//!`
// doc comment here is rejected by rustc with E0753 ("expected outer doc comment")
// because it lands part-way through the build script's crate root. The module
// documentation therefore lives on the `pub mod build_fingerprint;` declaration in
// `src/lib.rs`, and the trade-off is stated there too.

use std::path::{Path, PathBuf};

/// FNV-1a 64-bit offset basis, from the original FNV specification.
pub const BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fold `bytes` into an FNV-1a 64-bit hash, continuing from `seed`.
pub fn fold(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// The files whose content decides what the binary does.
///
/// Order is FIXED and is part of the digest: `Cargo.toml`, `Cargo.lock` (either
/// may be absent in an unusual checkout), then every `.rs` file under `src/`
/// sorted by relative path as a string, so the same tree always produces the same
/// sequence regardless of directory iteration order.
pub fn tracked_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = Vec::new();
    for name in ["Cargo.toml", "Cargo.lock"] {
        let p = root.join(name);
        if p.is_file() {
            out.push(p);
        }
    }
    let mut rs: Vec<PathBuf> = Vec::new();
    let src = root.join("src");
    if !src.is_dir() {
        return Err(format!(
            "no source directory at {} — the tree this binary was built from is not here",
            src.display()
        ));
    }
    collect_rs(&src, &mut rs)?;
    rs.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    out.extend(rs);
    Ok(out)
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot list {}: {}", dir.display(), e))?;
    for e in entries {
        let e = e.map_err(|err| format!("cannot read an entry of {}: {}", dir.display(), err))?;
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out)?;
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    Ok(())
}

/// Every directory the walk descends into, including `src` itself.
///
/// `build.rs` emits these as `cargo:rerun-if-changed` targets: a directory's mtime
/// moves when a file is ADDED or REMOVED, which is the only way a new source file
/// can be noticed on its own. Without them a file added after the last build would
/// be compiled (cargo tracks the crate's own sources) but never fingerprinted.
pub fn tracked_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = Vec::new();
    let src = root.join("src");
    if !src.is_dir() {
        return Err(format!("no source directory at {}", src.display()));
    }
    out.push(src.clone());
    let mut stack = vec![src];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.push(p.clone());
                stack.push(p);
            }
        }
    }
    out.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(out)
}

/// Fingerprint the tree at `root`, with the number of files that went into it (so a
/// caller can say how much of the tree it actually read).
///
/// Each file contributes: its relative path, a NUL, its length in bytes as 8
/// little-endian bytes, its content, and 0xFF. The length is in there so that two
/// files cannot be confused with one file whose name is the concatenation, and the
/// file COUNT is folded in first so that a tree with an extra empty file differs
/// from one without it.
pub fn fingerprint_with_count(root: &Path) -> Result<(String, usize), String> {
    let files = tracked_files(root)?;
    let mut h = fold(BASIS, &(files.len() as u64).to_le_bytes());
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .map_err(|_| format!("{} is not under {}", f.display(), root.display()))?;
        let rel = rel.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(f).map_err(|e| format!("cannot read {}: {}", f.display(), e))?;
        h = fold(h, rel.as_bytes());
        h = fold(h, &[0u8]);
        h = fold(h, &(bytes.len() as u64).to_le_bytes());
        h = fold(h, &bytes);
        h = fold(h, &[0xff]);
    }
    Ok((format!("{:016x}", h), files.len()))
}

/// The fingerprint alone, for callers that do not care how many files it read.
pub fn fingerprint(root: &Path) -> Result<String, String> {
    fingerprint_with_count(root).map(|(d, _)| d)
}

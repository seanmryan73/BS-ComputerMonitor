// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes

//! Every Rust source file carries the Author / Company header.
//!
//! Per `Author-Version-Standards.md` § Enforce it with a test. Until 2026-10-28 only `main.rs` and
//! `logging.rs` had one (2 of 15) and nothing checked. Copied from BS-Shooter-Rust, plus `build.rs`.
//!
//! - **The tree is walked off disk; no file is listed by hand.** A list can't notice the file nobody
//!   added to it.
//! - **It was watched failing before it was kept:** one header removed, the test went red.

use std::path::{Path, PathBuf};

/// How far into a file the header may sit. It belongs at the very top; a little slack keeps the test
/// from being defeated by a blank line.
const HEADER_WINDOW: usize = 6;

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|why| panic!("reading {}: {why}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// What is wrong with `text`'s header, or `None` when it is correct.
fn header_problem(text: &str) -> Option<String> {
    let top: Vec<&str> = text.lines().take(HEADER_WINDOW).collect();
    let find = |prefix: &str| top.iter().find_map(|line| line.strip_prefix(prefix)).map(str::trim);

    if find("// Author  :") != Some("Sean Ryan <seanmryan@gmail.com>") {
        return Some("missing or wrong `// Author  : Sean Ryan <seanmryan@gmail.com>`".into());
    }
    if find("// Company :") != Some("BagPipes") {
        return Some("missing or wrong `// Company : BagPipes`".into());
    }
    None
}

#[test]
fn every_rust_source_file_carries_the_standard_header() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["src", "tests"] {
        rust_files(&root.join(dir), &mut files);
    }
    files.push(root.join("build.rs"));
    assert!(
        files.len() > 15,
        "found only {} files - is the walk looking in the right place?",
        files.len()
    );

    let problems: Vec<String> = files
        .iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).expect("a readable source file");
            header_problem(&text).map(|why| format!("{}: {why}", path.strip_prefix(root).unwrap_or(path).display()))
        })
        .collect();

    assert!(
        problems.is_empty(),
        "files without the standard header:\n{}",
        problems.join("\n")
    );
}

#[test]
fn a_file_with_no_header_is_reported() {
    assert!(header_problem("//! just docs\nfn main() {}\n").is_some());
    assert!(header_problem("// Author  : Sean Ryan <seanmryan@gmail.com>\n// Company : BagPipes\n").is_none());
}

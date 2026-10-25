//! The Function entry points may not make a runtime of their own.
//!
//! Tokio refuses to start a runtime inside a runtime: `block_on` cannot block
//! a thread that is already driving an executor, so the second runtime
//! panics with "Cannot start a runtime from within a runtime" and the
//! Function dies on its first request.
//!
//! That is not hypothetical. The identity Function's entry point was
//! `#[tokio::main]` and it called into `serve.rs`, which makes the runtime
//! the Function actually serves on — so production answered 500 for every
//! session request until the entry point was made synchronous again.
//!
//! The runtime belongs to `services/deploy/vercel/src/serve.rs`. These tests
//! are the contract that says so, and they fail the moment somebody puts a
//! second one back.

use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
}

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(dir, &mut found);
    found.sort();
    found
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("could not read {}: {error}", dir.display());
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("could not read an entry of {}: {error}", dir.display());
        });
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn no_entry_point_makes_a_runtime_of_its_own() {
    let entries = rust_files_under(&repository_root().join("api"));
    assert!(
        !entries.is_empty(),
        "no entry point found under api/ — the deployment would serve nothing"
    );

    for entry in entries {
        let source = read(&entry);
        for forbidden in ["#[tokio::main]", "Runtime::new()", "block_on"] {
            assert!(
                !source.contains(forbidden),
                "{} contains `{forbidden}`. The runtime belongs to \
                 services/deploy/vercel/src/serve.rs: a runtime of its own \
                 inside the one that serves the Function makes tokio panic \
                 with \"Cannot start a runtime from within a runtime\", and \
                 the Function answers 500 instead of serving.",
                entry.display()
            );
        }
    }
}

#[test]
fn exactly_one_place_makes_the_runtime() {
    let deployment = repository_root().join("services/deploy/vercel/src");
    let makers: Vec<PathBuf> = rust_files_under(&deployment)
        .into_iter()
        .filter(|path| read(path).contains("Runtime::new()"))
        .collect();

    assert_eq!(
        makers.len(),
        1,
        "the runtime is made in exactly one place, but {} make one: {}",
        makers.len(),
        makers
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        makers[0].ends_with("serve.rs"),
        "the runtime is made in serve.rs, not in {}",
        makers[0].display()
    );
}

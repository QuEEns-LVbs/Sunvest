//! Shared context resolution for `cougr check` and `cougr check --verified`.
//!
//! Determines the repository root and which examples to check, supporting
//! auto-detection from cwd, explicit `--path`, and `--example` flags.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Metadata for a single example discovered under `examples/`.
#[derive(Clone, Debug)]
pub struct Example {
    pub name: String,
}

/// The resolved context for a check run.
pub struct CheckContext {
    pub repo_root: PathBuf,
    pub examples: Vec<Example>,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// Determine the repo root and which examples to check.
pub fn resolve(
    cwd: &Path,
    explicit_root: Option<&str>,
    single_example: Option<&str>,
) -> Result<CheckContext> {
    // 1. Determine repo root
    let repo_root = if let Some(r) = explicit_root {
        let p = PathBuf::from(r);
        p.canonicalize()
            .context(format!("explicit --path does not exist: {}", r))?
    } else {
        find_repo_root(cwd)?
    };

    // 2. Determine examples to check
    let examples = if let Some(name) = single_example {
        let example_dir = repo_root.join("examples").join(name);
        if !example_dir.is_dir() {
            anyhow::bail!("example '{}' not found at {}", name, example_dir.display());
        }
        vec![Example {
            name: name.to_string(),
        }]
    } else if explicit_root.is_none() && is_inside_example_dir(cwd) {
        // Auto-detect: if cwd is inside examples/<name>/, just check that one.
        // Only auto-detect when no explicit --path was given.
        let name = cwd
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .context("cannot determine example name from current directory")?;
        vec![Example { name }]
    } else {
        discover_examples(&repo_root)?
    };

    Ok(CheckContext {
        repo_root,
        examples,
    })
}

/// Return the absolute path to an example's directory.
pub fn example_dir(repo_root: &Path, name: &str) -> PathBuf {
    repo_root.join("examples").join(name)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk upward from `cwd` looking for a directory that contains both
/// `Cargo.toml` and an `examples/` subdirectory (the repo root).
fn find_repo_root(cwd: &Path) -> Result<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        if current.join("Cargo.toml").is_file() && current.join("examples").is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            anyhow::bail!(
                "could not find repo root (no Cargo.toml + examples/ found above {:?}). \
                 Use --path to specify the repo root explicitly.",
                cwd
            );
        }
    }
}

/// True when `cwd` is inside an `examples/<name>/` directory.
fn is_inside_example_dir(cwd: &Path) -> bool {
    if let Some(parent) = cwd.parent() {
        parent.file_name().map(|n| n == "examples").unwrap_or(false)
    } else {
        false
    }
}

/// Discover all example directories under `examples/` that contain a Cargo.toml.
fn discover_examples(repo_root: &Path) -> Result<Vec<Example>> {
    let examples_dir = repo_root.join("examples");
    let mut examples = Vec::new();

    if !examples_dir.is_dir() {
        anyhow::bail!(
            "examples/ directory not found at {}",
            examples_dir.display()
        );
    }

    for entry in fs::read_dir(&examples_dir).context("cannot read examples/ directory")? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_name = entry.file_name();
            if let Some(name) = dir_name.to_str() {
                if entry.path().join("Cargo.toml").is_file() {
                    examples.push(Example {
                        name: name.to_string(),
                    });
                }
            }
        }
    }

    if examples.is_empty() {
        anyhow::bail!("no examples found under {}", examples_dir.display());
    }

    Ok(examples)
}

// ---------------------------------------------------------------------------
// Canonical examples (from examples/catalog.toml)
// ---------------------------------------------------------------------------

/// Minimal representation of `examples/catalog.toml`, mirroring the schema
/// validated by `scripts/validate_catalog.py`.
#[derive(Debug, Deserialize)]
struct Catalog {
    #[serde(default)]
    example: std::collections::BTreeMap<String, CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    name: String,
    #[serde(default)]
    maturity: Option<String>,
}

/// Return the names of all examples marked `maturity = "canonical"` in
/// `examples/catalog.toml`. The catalog is the single source of truth for
/// example metadata; promoting an example is a one-line catalog change.
///
/// Returns an error if the catalog is missing, unreadable, or malformed.
pub fn canonical_example_names(repo_root: &Path) -> Result<Vec<String>> {
    let catalog_path = repo_root.join("examples").join("catalog.toml");
    let contents = fs::read_to_string(&catalog_path).with_context(|| {
        format!(
            "cannot read example catalog at {} (expected examples/catalog.toml)",
            catalog_path.display()
        )
    })?;

    let catalog: Catalog = toml::from_str(&contents).with_context(|| {
        format!(
            "failed to parse example catalog at {}",
            catalog_path.display()
        )
    })?;

    let mut names: Vec<String> = catalog
        .example
        .into_iter()
        .filter_map(|(_key, entry)| {
            let is_canonical = entry
                .maturity
                .as_deref()
                .map(|m| m.eq_ignore_ascii_case("canonical"))
                .unwrap_or(false);
            if is_canonical {
                Some(entry.name)
            } else {
                None
            }
        })
        .collect();

    names.sort();
    names.dedup();

    if names.is_empty() {
        anyhow::bail!(
            "no examples with maturity = \"canonical\" found in {}",
            catalog_path.display()
        );
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_catalog(dir: &Path, body: &str) {
        let examples = dir.join("examples");
        fs::create_dir_all(&examples).unwrap();
        let mut f = fs::File::create(examples.join("catalog.toml")).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn canonical_example_names_filters_by_maturity() {
        let tmp = std::env::temp_dir().join(format!(
            "cougr-catalog-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        write_catalog(
            &tmp,
            r#"
[example.spawn_and_move]
name = "spawn_and_move"
category = "basics"
maturity = "canonical"
cougr_features = []

[example.tic_tac_toe]
name = "tic_tac_toe"
category = "games"
maturity = "canonical"
cougr_features = []

[example.wip_thing]
name = "wip_thing"
category = "games"
maturity = "transitional"
cougr_features = []
"#,
        );

        let names = canonical_example_names(&tmp).unwrap();
        assert_eq!(
            names,
            vec!["spawn_and_move".to_string(), "tic_tac_toe".to_string()]
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn canonical_example_names_errors_on_missing_catalog() {
        let tmp = std::env::temp_dir().join(format!(
            "cougr-catalog-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let err = canonical_example_names(&tmp).unwrap_err();
        assert!(format!("{err}").contains("catalog.toml"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn canonical_example_names_errors_on_malformed_catalog() {
        let tmp = std::env::temp_dir().join(format!(
            "cougr-catalog-malformed-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        write_catalog(&tmp, "this is not valid toml [[[");

        let err = canonical_example_names(&tmp).unwrap_err();
        assert!(format!("{err}").contains("failed to parse"));

        let _ = fs::remove_dir_all(&tmp);
    }
}

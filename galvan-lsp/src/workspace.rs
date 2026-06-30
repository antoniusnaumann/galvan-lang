//! Crate-wide semantic index.
//!
//! A Galvan crate is a `src/` directory whose `.galvan` files share a single
//! name namespace (the compiler aggregates them the same way — see
//! `galvan_files::read_sources` and `LookupContext::add_from`). A [`Crate`]
//! loads every file in that directory so that name resolution — and therefore
//! hover and go-to-definition — works across files within the crate.
//!
//! Files that are open in the editor are taken from their in-memory buffer
//! (honouring unsaved edits); the rest are read from disk.

use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use galvan_ast::{Ast, SegmentedAsts};
use galvan_files::{read_sources, Source};
use galvan_hir::{typecheck, Diagnostic};
use galvan_into_ast::{SegmentAst, SourceIntoAst};
use galvan_resolver::LookupContext;
use tower_lsp::lsp_types::Url;

use crate::document::Document;

/// A single file's parsed contents within a crate.
pub struct CrateFile {
    pub source: Source,
    pub segmented: Option<SegmentedAsts>,
}

/// All files belonging to one Galvan crate, parsed and ready for resolution.
pub struct Crate {
    files: Vec<CrateFile>,
}

impl Crate {
    /// Load the crate that `uri` belongs to.
    ///
    /// All `.galvan` files under the crate's source root are read from disk,
    /// except those currently open in the editor, which are taken from their
    /// in-memory buffers in `open` so that unsaved edits are reflected.
    pub fn load(uri: &Url, open: &DashMap<Url, Document>) -> Self {
        let mut sources: Vec<Source> = Vec::new();

        if let Ok(path) = uri.to_file_path() {
            let root = crate_root(&path);
            sources = read_sources(&root, vec![]).unwrap_or_default();

            for entry in open.iter() {
                let Ok(buffer_path) = entry.key().to_file_path() else {
                    continue;
                };
                if buffer_path.starts_with(&root) {
                    upsert(&mut sources, file_source(&buffer_path, entry.value().text()));
                }
            }
        } else if let Some(doc) = open.get(uri) {
            // Non-file documents (e.g. untitled buffers) resolve in isolation.
            sources.push(Source::from_string(doc.text().to_string()));
        }

        Self::from_sources(sources)
    }

    /// Build a crate directly from a set of (absolute path, contents) pairs,
    /// without touching the filesystem. Intended for tests.
    pub fn in_memory(files: impl IntoIterator<Item = (PathBuf, String)>) -> Self {
        let sources = files
            .into_iter()
            .map(|(path, content)| file_source(&path, &content))
            .collect();
        Self::from_sources(sources)
    }

    fn from_sources(sources: Vec<Source>) -> Self {
        let files = sources
            .into_iter()
            .map(|source| {
                let segmented = source
                    .clone()
                    .try_into_ast()
                    .and_then(SegmentAst::segmented)
                    .ok();
                CrateFile { source, segmented }
            })
            .collect();
        Self { files }
    }

    /// Build a combined lookup context spanning every file in the crate.
    ///
    /// Duplicate declarations (themselves diagnostics) are tolerated: resolution
    /// stays partial rather than failing outright.
    pub fn lookup(&self) -> LookupContext<'_> {
        let mut lookup = LookupContext::new();
        for file in &self.files {
            if let Some(segmented) = &file.segmented {
                let _ = lookup.add_from(segmented);
            }
        }
        lookup
    }

    pub fn files(&self) -> impl Iterator<Item = &CrateFile> {
        self.files.iter()
    }

    /// Typecheck the whole crate and return the compiler's diagnostics.
    ///
    /// The crate is checked as a unit so that cross-file references resolve;
    /// each returned [`Diagnostic`]'s span carries the file it belongs to (see
    /// the `set_current_file` mechanism in `galvan-hir`), letting callers route
    /// it back to the right document.
    ///
    /// Diagnostics are not produced for crates that fail to parse or that have
    /// conflicting top-level declarations (a [`LookupError`](galvan_resolver::LookupError)).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let asts: Vec<Ast> = self
            .files
            .iter()
            .filter_map(|file| file.source.clone().try_into_ast().ok())
            .collect();
        let Ok(segmented) = asts.segmented() else {
            return Vec::new();
        };

        // Guard against the typechecker panicking on pathological input: a
        // language server must keep running whatever the buffer contains.
        let checked = std::panic::catch_unwind(AssertUnwindSafe(|| typecheck(segmented)));
        match checked {
            Ok(Ok((_module, errors))) => errors.diagnostics().to_vec(),
            _ => Vec::new(),
        }
    }
}

/// The source root of the crate containing `file`: the nearest ancestor
/// directory named `src` (matching the compiler, which transpiles `src`). If
/// the file is not inside a `src` directory, its own directory is used so that
/// loose files still resolve against their siblings.
fn crate_root(file: &Path) -> PathBuf {
    for ancestor in file.ancestors() {
        if ancestor.file_name() == Some("src".as_ref()) {
            return ancestor.to_path_buf();
        }
    }
    file.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Construct a file-backed [`Source`] from in-memory contents, mirroring how
/// `Source::read` derives the canonical name (without reading from disk).
fn file_source(path: &Path, content: &str) -> Source {
    let canonical_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace('.', "_"))
        .unwrap_or_default();

    Source::File {
        path: Arc::from(path.to_path_buf()),
        content: Arc::from(content),
        canonical_name: Arc::from(canonical_name),
    }
}

/// Insert `source`, replacing any existing entry for the same file path.
fn upsert(sources: &mut Vec<Source>, source: Source) {
    if let Some(path) = source.origin().map(Path::to_path_buf) {
        if let Some(slot) = sources
            .iter_mut()
            .find(|s| s.origin() == Some(path.as_path()))
        {
            *slot = source;
            return;
        }
    }
    sources.push(source);
}

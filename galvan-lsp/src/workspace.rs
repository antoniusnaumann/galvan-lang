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

use std::collections::BTreeSet;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use galvan_ast::{Ast, SegmentedAsts};
use galvan_files::{read_sources, Source};
use galvan_hir::{typecheck_with_interop, Diagnostic, HirModule, SymbolIndex};
use galvan_into_ast::{SegmentAst, SourceIntoAst};
use galvan_resolver::LookupContext;
use galvan_rustdoc::RustInterop;
use tower_lsp::lsp_types::Url;

use crate::document::Document;

/// The result of typechecking a whole crate: the typed HIR, the
/// position-indexed symbol information recorded by the typechecker, and the
/// crate-wide diagnostics.
pub struct Analysis {
    pub module: HirModule,
    pub index: SymbolIndex,
    pub diagnostics: Vec<Diagnostic>,
    /// The consumer Cargo project the Rust interop was resolved against, when
    /// one was found. Relative Rust source paths (go-to-definition into Rust)
    /// resolve against this directory.
    pub manifest_dir: Option<PathBuf>,
}

/// Process-wide cache of built Rust interops, keyed by the consumer project
/// and the set of crates the Galvan sources mention in `use` declarations.
/// Building an interop can run `cargo metadata` and rustdoc (slow); analyze()
/// runs per keystroke and must not. The cache is only invalidated by a new
/// `use` (new key); dependency changes are picked up on server restart.
fn interop_cache() -> &'static DashMap<(PathBuf, BTreeSet<String>), Arc<RustInterop>> {
    static CACHE: OnceLock<DashMap<(PathBuf, BTreeSet<String>), Arc<RustInterop>>> =
        OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

/// The directory of the Cargo project containing `file`: the nearest ancestor
/// with a `Cargo.toml`.
fn cargo_manifest_dir(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

/// A single file's parsed contents within a crate.
pub struct CrateFile {
    pub source: Source,
    pub segmented: Option<SegmentedAsts>,
}

/// All files belonging to one Galvan crate, parsed and ready for resolution.
pub struct Crate {
    files: Vec<CrateFile>,
    /// Memoized result of [`Crate::analyze`]: the crate is typechecked at
    /// most once per `Crate` instance, however many features ask for it.
    analysis: OnceLock<Option<Analysis>>,
    /// Interop to analyze with instead of resolving one from the crate's
    /// `use` declarations — lets tests exercise the interop features without
    /// running cargo/rustdoc.
    interop_override: Option<Arc<RustInterop>>,
}

impl Crate {
    /// Load the crate that `uri` belongs to.
    ///
    /// All `.galvan` files under the crate's source root are read from disk,
    /// except those currently open in the editor, which are taken from their
    /// in-memory buffers in `open` so that unsaved edits are reflected.
    pub fn load(uri: &Url, open: &DashMap<Url, Document>) -> Self {
        let mut sources: Vec<Source> = Vec::new();

        if let Some(root) = uri.to_file_path().ok().as_deref().and_then(crate_root) {
            sources = read_sources(&root, vec![]).unwrap_or_else(|error| {
                // Stderr goes to the client's server log.
                eprintln!("galvan-lsp: failed to read crate sources at {root:?}: {error}");
                Vec::new()
            });

            for entry in open.iter() {
                let Ok(buffer_path) = entry.key().to_file_path() else {
                    continue;
                };
                if buffer_path.starts_with(&root) {
                    upsert(&mut sources, file_source(&buffer_path, entry.value().text()));
                }
            }
        } else if let Some(doc) = open.get(uri) {
            // Non-file documents (e.g. untitled buffers) and files without a
            // resolvable crate root are analyzed in isolation.
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

    /// [`Crate::in_memory`] with a fixed Rust interop, bypassing cargo and
    /// rustdoc. Intended for tests of the interop-backed features.
    pub fn in_memory_with_interop(
        files: impl IntoIterator<Item = (PathBuf, String)>,
        interop: RustInterop,
    ) -> Self {
        let mut krate = Self::in_memory(files);
        krate.interop_override = Some(Arc::new(interop));
        krate
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
        Self {
            files,
            analysis: OnceLock::new(),
            interop_override: None,
        }
    }

    /// Whether `file` parses (and therefore participates in [`Crate::analyze`]).
    /// Files that fail to parse are silently absent from the analysis, so
    /// features must fall back to probes or name-based resolution for them.
    pub fn file_parses(&self, file: &Path) -> bool {
        self.files
            .iter()
            .any(|f| f.source.origin() == Some(file) && f.segmented.is_some())
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

    /// A copy of this crate with the contents of `file` replaced by `text`
    /// (used to analyze completion probes without mutating the real crate).
    pub fn with_file_text(&self, file: &Path, text: &str) -> Self {
        let sources = self
            .files
            .iter()
            .map(|crate_file| {
                if crate_file.source.origin() == Some(file) {
                    file_source(file, text)
                } else {
                    crate_file.source.clone()
                }
            })
            .collect();
        let mut probe = Self::from_sources(sources);
        probe.interop_override = self.interop_override.clone();
        probe
    }

    /// Typecheck the whole crate.
    ///
    /// The crate is checked as a unit so that cross-file references resolve;
    /// each returned [`Diagnostic`]'s span carries the file it belongs to (see
    /// the `set_current_file` mechanism in `galvan-hir`), letting callers route
    /// it back to the right document. The returned [`Analysis`] also carries
    /// the typechecker's [`SymbolIndex`] and the typed [`HirModule`], which
    /// power hover, go-to-definition, references and completion.
    ///
    /// Returns `None` when the crate does not parse (analysis then degrades
    /// gracefully; syntax errors are reported separately).
    ///
    /// The result is computed once per `Crate` instance and memoized, so
    /// features may call this freely.
    pub fn analyze(&self) -> Option<&Analysis> {
        self.analysis
            .get_or_init(|| self.run_analysis())
            .as_ref()
    }

    fn run_analysis(&self) -> Option<Analysis> {
        let asts: Vec<Ast> = self
            .files
            .iter()
            .filter_map(|file| file.source.clone().try_into_ast().ok())
            .collect();
        let segmented = asts.segmented().ok()?;
        let (interop, manifest_dir) = self.rust_interop(&segmented);

        // Guard against the typechecker panicking on pathological input: a
        // language server must keep running whatever the buffer contains.
        let checked = std::panic::catch_unwind(AssertUnwindSafe(|| {
            typecheck_with_interop(segmented, &interop)
        }))
        .map_err(|panic| {
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .map(str::to_owned)
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic>".to_string());
            eprintln!("galvan-lsp: typechecker panicked: {message}");
        })
        .ok()?;
        Some(Analysis {
            module: checked.module,
            diagnostics: checked.errors.diagnostics().to_vec(),
            index: checked.index,
            manifest_dir,
        })
    }

    /// The Rust interop for this crate's `use` declarations, built once per
    /// (project, imported crates) pair and shared process-wide. Crates without
    /// a Cargo project or without `use` declarations get an empty interop.
    fn rust_interop(&self, segmented: &SegmentedAsts) -> (Arc<RustInterop>, Option<PathBuf>) {
        if let Some(interop) = &self.interop_override {
            return (interop.clone(), None);
        }
        let manifest_dir = self
            .files
            .iter()
            .filter_map(|file| file.source.origin())
            .find_map(cargo_manifest_dir);
        let Some(manifest_dir) = manifest_dir else {
            return (Arc::new(RustInterop::empty()), None);
        };
        let crates: BTreeSet<String> = segmented
            .uses
            .iter()
            .filter_map(|decl| decl.item.path.segments.first())
            .map(|segment| segment.as_str().to_owned())
            .collect();
        if crates.is_empty() {
            return (Arc::new(RustInterop::empty()), Some(manifest_dir));
        }

        let key = (manifest_dir.clone(), crates);
        let interop = interop_cache()
            .entry(key)
            .or_insert_with(|| {
                let interop =
                    RustInterop::from_uses_in(&manifest_dir, &segmented.uses, |warning| {
                        eprintln!("galvan-lsp: rustdoc interop: {warning}");
                    })
                    .unwrap_or_else(|error| {
                        eprintln!("galvan-lsp: rustdoc interop unavailable: {error}");
                        RustInterop::empty()
                    });
                Arc::new(interop)
            })
            .clone();
        (interop, Some(manifest_dir))
    }
}

/// The source root of the crate containing `file`: the nearest ancestor
/// directory named `src` (matching the compiler, which transpiles `src`). If
/// the file is not inside a `src` directory, its own directory is used so that
/// loose files still resolve against their siblings. `None` when the file has
/// no parent directory (never a directory of the server's own choosing).
pub fn crate_root(file: &Path) -> Option<PathBuf> {
    for ancestor in file.ancestors() {
        if ancestor.file_name() == Some("src".as_ref()) {
            return Some(ancestor.to_path_buf());
        }
    }
    file.parent().map(Path::to_path_buf)
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

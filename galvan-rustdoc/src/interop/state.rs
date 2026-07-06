use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl};
use galvan_files::Source;
use serde_json::Value;

use crate::cache::RustdocCache;
use crate::model::{RustConstantDecl, RustFunctionDecl, RustReturnConversion, RustTypeDecl};
use crate::RustdocError;

use super::function_id::RustFunctionId;
use super::uses::imported_crates;

/// rustdoc JSON format version this crate was written against.
///
/// rustdoc JSON is explicitly unstable and versioned: every schema this crate
/// deserializes into (the `rustdoc_types` structs) is valid only for this exact
/// `format_version`. A toolchain that emits a different version produces JSON
/// that either fails to deserialize or drops items, so we assert the version up
/// front and fail loudly instead of silently mis-parsing.
///
/// This is one half of a coordinated pin: the `rustdoc-types` dependency
/// (`=0.60.0`, whose `FORMAT_VERSION` is `60`) must stay in lockstep with the
/// nightly toolchain the cache is generated with (see
/// [`crate::cache`] / `GALVAN_RUSTDOC_TOOLCHAIN`). Bumping to a new schema means
/// bumping *both*: the `rustdoc-types` version here and the pinned nightly date.
pub(super) const RUSTDOC_FORMAT_VERSION: u64 = rustdoc_types::FORMAT_VERSION as u64;

/// Whether a `(receiver, name/id)` associated item is exposed by exactly one
/// namespace (`One`) or by more than one (`Many`). Used by the unqualified
/// associated lookups to answer "is this unambiguous?" in O(1) instead of
/// scanning every namespaced associated item.
#[derive(Debug)]
pub(super) enum Unambiguous {
    /// Unique so far: the sole exposing namespace and the resolved index. If the
    /// same namespace re-inserts the pair (a duplicate that overwrites the
    /// namespaced map) we keep the latest index and stay unambiguous.
    One { namespace: String, idx: usize },
    /// Two or more distinct namespaces expose the pair, so it is ambiguous.
    Many,
}

#[derive(Debug, Default)]
pub struct RustInterop {
    pub types: Vec<RustTypeDecl>,
    pub functions: Vec<RustFunctionDecl>,
    pub constants: Vec<RustConstantDecl>,
    pub(super) by_imported_type: HashMap<TypeIdent, usize>,
    pub(super) by_namespace_function: HashMap<(String, RustFunctionId), usize>,
    pub(super) by_imported_function: HashMap<RustFunctionId, usize>,
    pub(super) by_namespace_associated_function:
        HashMap<(String, TypeIdent, RustFunctionId), usize>,
    pub(super) by_associated_function: HashMap<(TypeIdent, RustFunctionId), Unambiguous>,
    pub(super) by_namespace_constant: HashMap<(String, Ident), usize>,
    pub(super) by_imported_constant: HashMap<Ident, usize>,
    pub(super) by_namespace_associated_constant: HashMap<(String, TypeIdent, Ident), usize>,
    pub(super) by_associated_constant: HashMap<(TypeIdent, Ident), Unambiguous>,
}

impl RustInterop {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_uses(uses: &[ToplevelItem<UseDecl>]) -> Result<Self, RustdocError> {
        Self::from_crates_and_uses(imported_crates(uses), uses)
    }

    pub fn from_crates_and_uses(
        crate_names: impl IntoIterator<Item = String>,
        uses: &[ToplevelItem<UseDecl>],
    ) -> Result<Self, RustdocError> {
        let mut interop = RustInterop::default();
        let imported_crates = imported_crates(uses);
        let crate_names = crate_names
            .into_iter()
            .chain(imported_crates.iter().cloned())
            .collect::<HashSet<_>>();

        for crate_name in crate_names {
            let cache = RustdocCache::new(&crate_name);
            cache.update_if_needed();
            if let Some(path) = cache.json_path() {
                let text = fs::read_to_string(&path)
                    .map_err(|error| RustdocError::ReadCache(path.clone(), error))?;
                let json: Value = serde_json::from_str(&text)
                    .map_err(|error| RustdocError::ParseCache(path.clone(), error))?;
                // Reject a drifted schema up front with an actionable error before
                // attempting the typed deserialization (which would otherwise fail
                // with an opaque serde message).
                check_format_version(&path, &json)?;
                let krate: rustdoc_types::Crate = serde_json::from_value(json)
                    .map_err(|error| RustdocError::ParseCache(path.clone(), error))?;
                interop.add_crate(&crate_name, &krate);
            }
        }
        interop.import_uses(uses);

        Ok(interop)
    }

    pub fn add_function_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            RustReturnConversion::None,
            Vec::new(),
        );
    }

    pub fn add_type_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: TypeDecl,
    ) {
        let rust_path = rust_path.into();
        let type_decl = RustTypeDecl {
            namespace: namespace.into(),
            name: TypeIdent::new(name),
            rust_path,
            field_conversions: Vec::new(),
            constructor_arg_conversions: Vec::new(),
            enum_variant_conversions: Vec::new(),
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        };
        if let Some(existing) = self
            .types
            .iter_mut()
            .find(|ty| ty.rust_path.as_ref() == type_decl.rust_path.as_ref())
        {
            *existing = type_decl;
        } else {
            self.types.push(type_decl);
        }
    }

    pub fn add_constant_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, None, name, rust_path.into(), ty);
    }

    pub fn add_associated_constant_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        ty: TypeElement,
    ) {
        self.push_constant(namespace, Some(receiver), name, rust_path.into(), ty);
    }

    pub fn add_associated_function_decl(
        &mut self,
        namespace: &str,
        receiver: TypeIdent,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function_with_associated_receiver(
            namespace,
            name,
            rust_path.into(),
            decl,
            borrowed_return,
            Some(receiver),
            RustReturnConversion::None,
            Vec::new(),
        );
    }
}

/// Reject a rustdoc JSON cache whose `format_version` does not match the schema
/// this crate walks, so a nightly toolchain bump fails loudly instead of
/// silently dropping every item it can no longer parse.
fn check_format_version(path: &Path, json: &Value) -> Result<(), RustdocError> {
    let Some(version) = json.get("format_version").and_then(Value::as_u64) else {
        return Err(RustdocError::MissingFormatVersion(path.to_path_buf()));
    };
    if version != RUSTDOC_FORMAT_VERSION {
        return Err(RustdocError::UnsupportedFormatVersion(
            path.to_path_buf(),
            version,
            RUSTDOC_FORMAT_VERSION,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod format_version_tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn path() -> PathBuf {
        PathBuf::from("cache.json")
    }

    #[test]
    fn accepts_matching_version() {
        let json = json!({ "format_version": RUSTDOC_FORMAT_VERSION, "index": {} });
        assert!(check_format_version(&path(), &json).is_ok());
    }

    #[test]
    fn rejects_mismatched_version() {
        let json = json!({ "format_version": RUSTDOC_FORMAT_VERSION + 1, "index": {} });
        let error = check_format_version(&path(), &json).unwrap_err();
        assert!(matches!(
            error,
            RustdocError::UnsupportedFormatVersion(_, actual, expected)
                if actual == RUSTDOC_FORMAT_VERSION + 1 && expected == RUSTDOC_FORMAT_VERSION
        ));
    }

    #[test]
    fn rejects_missing_version() {
        let json = json!({ "index": {} });
        let error = check_format_version(&path(), &json).unwrap_err();
        assert!(matches!(error, RustdocError::MissingFormatVersion(_)));
    }
}

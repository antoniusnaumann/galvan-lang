use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::{env, fs};

use serde_json::Value;

use galvan_ast::{FnDecl, Ident, ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl};
use galvan_files::Source;

use crate::cache::RustdocCache;
use crate::model::{
    RustConstantDecl, RustFunctionDecl, RustReturnConversion, RustTypeDecl, RustdocCrateLiftSummary,
};
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
/// [`crate::cache`] / `GALVAN_RUSTDOC_TOOLCHAIN` / `GALVAN_RUSTDOC_COMMAND`).
/// Bumping to a new schema means bumping *both*: the `rustdoc-types` version
/// here and the pinned nightly date.
pub(super) const RUSTDOC_FORMAT_VERSION: u64 = rustdoc_types::FORMAT_VERSION as u64;
const RUSTDOC_REQUIRE_LIFT_ENV: &str = "GALVAN_RUSTDOC_REQUIRE_LIFT";

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
    pub(super) lift_summaries: HashMap<Box<str>, RustdocCrateLiftSummary>,
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
        Self::from_crates_and_uses_with_warnings(crate_names, uses, |_| {})
    }

    pub fn from_crates_and_uses_with_warnings(
        crate_names: impl IntoIterator<Item = String>,
        uses: &[ToplevelItem<UseDecl>],
        warn: impl FnMut(&RustdocError),
    ) -> Result<Self, RustdocError> {
        Self::from_crates_and_uses_with_options(
            crate_names,
            uses,
            warn,
            require_lift_from_env(env::var(RUSTDOC_REQUIRE_LIFT_ENV).ok().as_deref()),
        )
    }

    fn from_crates_and_uses_with_options(
        crate_names: impl IntoIterator<Item = String>,
        uses: &[ToplevelItem<UseDecl>],
        mut warn: impl FnMut(&RustdocError),
        require_lift: bool,
    ) -> Result<Self, RustdocError> {
        let mut interop = RustInterop::default();
        let imported_crates = imported_crates(uses);
        let crate_names = crate_names
            .into_iter()
            .chain(imported_crates.iter().cloned())
            .collect::<HashSet<_>>();
        let mut warned = HashSet::new();

        for crate_name in crate_names {
            let cache = RustdocCache::new(&crate_name);
            match cache.update_if_needed() {
                Ok(()) => {}
                Err(error) if is_soft_cache_error(&error) => {
                    if require_lift {
                        return Err(error);
                    }
                    warn_soft_cache_error_once(&error, &mut warned, &mut warn);
                    continue;
                }
                Err(error) => return Err(error),
            }
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
                let summary = interop.add_crate(&crate_name, &krate);
                if require_lift && summary.total_items() == 0 {
                    return Err(RustdocError::RequiredLiftEmpty(summary.crate_name));
                }
            } else if require_lift {
                return Err(RustdocError::RequiredLiftEmpty(crate_name.into()));
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

    pub fn lift_summary(&self, crate_name: &str) -> Option<&RustdocCrateLiftSummary> {
        self.lift_summaries.get(crate_name)
    }

    pub fn lift_summaries(&self) -> impl Iterator<Item = &RustdocCrateLiftSummary> {
        self.lift_summaries.values()
    }
}

fn is_soft_cache_error(error: &RustdocError) -> bool {
    matches!(
        error,
        RustdocError::DependencyNotFound(_)
            | RustdocError::ToolchainUnavailable
            | RustdocError::ToolchainNotInstalled(_)
            | RustdocError::RustdocSpawn(_)
    )
}

fn warn_soft_cache_error_once(
    error: &RustdocError,
    warned: &mut HashSet<String>,
    warn: &mut impl FnMut(&RustdocError),
) {
    if !matches!(
        error,
        RustdocError::ToolchainUnavailable
            | RustdocError::ToolchainNotInstalled(_)
            | RustdocError::RustdocSpawn(_)
    ) {
        return;
    }

    let message = error.to_string();
    if warned.insert(message) {
        warn(error);
    }
}

fn require_lift_from_env(value: Option<&str>) -> bool {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };

    value != "0" && !value.eq_ignore_ascii_case("false")
}

/// Reject a rustdoc JSON cache whose `format_version` does not match the schema
/// this crate walks, so a nightly toolchain bump fails loudly instead of
/// silently dropping every item it can no longer parse.
pub fn check_format_version(path: &Path, json: &Value) -> Result<(), RustdocError> {
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

    #[test]
    fn soft_cache_warnings_are_callback_driven_and_deduplicated() {
        let mut warned = HashSet::new();
        let mut warnings = Vec::new();
        let mut warn = |error: &RustdocError| warnings.push(error.to_string());

        warn_soft_cache_error_once(&RustdocError::ToolchainUnavailable, &mut warned, &mut warn);
        warn_soft_cache_error_once(&RustdocError::ToolchainUnavailable, &mut warned, &mut warn);
        warn_soft_cache_error_once(
            &RustdocError::DependencyNotFound("missing".into()),
            &mut warned,
            &mut warn,
        );

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("rustup was not found"));
    }

    #[test]
    fn require_lift_env_treats_only_empty_zero_and_false_as_disabled() {
        assert!(!require_lift_from_env(None));
        assert!(!require_lift_from_env(Some("")));
        assert!(!require_lift_from_env(Some("0")));
        assert!(!require_lift_from_env(Some("false")));
        assert!(require_lift_from_env(Some("1")));
        assert!(require_lift_from_env(Some("true")));
    }

    #[test]
    fn require_lift_promotes_soft_cache_errors() {
        let error = RustInterop::from_crates_and_uses_with_options(
            ["definitely_missing_galvan_dep".to_string()],
            &[],
            |_| {},
            true,
        )
        .expect_err("required lift should reject missing dependency");

        assert!(matches!(error, RustdocError::DependencyNotFound(_)));
    }
}

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use thiserror::Error;

use galvan_ast::{
    BasicTypeItem, EmptyTypeDecl, FnDecl, FnSignature, Ident, Param, ParamList, ResultTypeItem,
    Span, ToplevelItem, TypeDecl, TypeElement, TypeIdent, UseDecl, Visibility,
};
use galvan_files::Source;

#[derive(Debug, Error)]
pub enum RustdocError {
    #[error("failed to run cargo metadata: {0}")]
    CargoMetadata(std::io::Error),
    #[error("cargo metadata returned invalid JSON: {0}")]
    InvalidCargoMetadata(serde_json::Error),
    #[error("failed to read rustdoc JSON cache {0}: {1}")]
    ReadCache(PathBuf, std::io::Error),
    #[error("failed to parse rustdoc JSON cache {0}: {1}")]
    ParseCache(PathBuf, serde_json::Error),
}

#[derive(Debug, Default)]
pub struct RustInterop {
    pub types: Vec<RustTypeDecl>,
    pub functions: Vec<RustFunctionDecl>,
    by_namespace_function: HashMap<(String, RustFunctionId), usize>,
    by_imported_function: HashMap<(String, RustFunctionId), usize>,
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
                let json = fs::read_to_string(&path)
                    .map_err(|error| RustdocError::ReadCache(path.clone(), error))?;
                let json = serde_json::from_str(&json)
                    .map_err(|error| RustdocError::ParseCache(path.clone(), error))?;
                interop.add_crate(&crate_name, &json);
            } else {
                interop.add_curated_crate(&crate_name);
            }
        }
        interop.import_uses(uses);

        Ok(interop)
    }

    pub fn add_crate(&mut self, crate_name: &str, json: &Value) {
        let Some(index) = json.get("index").and_then(Value::as_object) else {
            self.add_curated_crate(crate_name);
            return;
        };

        let mut type_names = HashSet::new();
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            if let Some(name) = public_type_name(item) {
                type_names.insert(name.to_string());
            }
        }

        for name in type_names {
            self.push_type(crate_name, &name);
        }

        let mut found_function = false;
        for item in index.values() {
            if !is_public(item) {
                continue;
            }
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(function) = inner(item, "function") else {
                continue;
            };
            let Some(signature) = function.get("sig") else {
                continue;
            };
            let rust_path = rust_path(crate_name, name, item);
            let decl = self.function_decl(crate_name, name, signature);
            let borrowed_return = return_is_borrowed(signature);
            self.push_function(crate_name, name, rust_path, decl, borrowed_return);
            found_function = true;
        }

        if !found_function {
            self.add_curated_crate(crate_name);
        }
    }

    pub fn add_function_decl(
        &mut self,
        namespace: &str,
        name: &str,
        rust_path: impl Into<Box<str>>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        self.push_function(namespace, name, rust_path.into(), decl, borrowed_return);
    }

    pub fn function(
        &self,
        namespace: Option<&str>,
        receiver: Option<&TypeIdent>,
        name: &Ident,
        labels: &[&str],
    ) -> Option<&RustFunctionDecl> {
        let id = RustFunctionId::new(receiver, name.as_str(), labels);
        if let Some(namespace) = namespace {
            return self
                .by_namespace_function
                .get(&(namespace.to_string(), id))
                .and_then(|idx| self.functions.get(*idx));
        }

        self.by_imported_function
            .get(&("".to_string(), id))
            .and_then(|idx| self.functions.get(*idx))
    }

    fn push_type(&mut self, crate_name: &str, name: &str) {
        if self.types.iter().any(|ty| ty.name.as_str() == name) {
            return;
        }

        let ident = TypeIdent::new(name);
        self.types.push(RustTypeDecl {
            name: ident.clone(),
            rust_path: format!("::{crate_name}::{name}").into(),
            decl: ToplevelItem {
                item: TypeDecl::Empty(EmptyTypeDecl {
                    visibility: Visibility::public(),
                    ident,
                    span: Span::default(),
                }),
                source: Source::Builtin,
            },
        });
    }

    fn push_function(
        &mut self,
        crate_name: &str,
        name: &str,
        rust_path: Box<str>,
        decl: FnDecl,
        borrowed_return: bool,
    ) {
        let labels = decl.signature.overload_labels();
        let labels = labels
            .iter()
            .map(|label| label.as_str())
            .collect::<Vec<_>>();
        let receiver = decl
            .signature
            .receiver()
            .and_then(|param| match &param.param_type {
                TypeElement::Plain(plain) => Some(&plain.ident),
                TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                _ => None,
            });
        let id = RustFunctionId::new(receiver, name, &labels);
        let idx = self.functions.len();
        self.functions.push(RustFunctionDecl {
            namespace: crate_name.into(),
            rust_path,
            borrowed_return,
            decl: ToplevelItem {
                item: decl,
                source: Source::Builtin,
            },
        });
        self.by_namespace_function
            .insert((crate_name.to_string(), id.clone()), idx);
    }

    fn import_uses(&mut self, uses: &[ToplevelItem<UseDecl>]) {
        for use_decl in uses {
            let Some(namespace) = use_decl.path.segments.first() else {
                continue;
            };
            let namespace = namespace.as_str();
            match use_decl.path.segments.as_slice() {
                [_] => self.import_namespace(namespace),
                [_, item] => self.import_item(namespace, item.as_str()),
                _ => {}
            }
        }
    }

    fn import_namespace(&mut self, namespace: &str) {
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
    }

    fn import_item(&mut self, namespace: &str, name: &str) {
        for (idx, function) in self.functions.iter().enumerate() {
            if function.namespace.as_ref() != namespace {
                continue;
            }
            let signature = &function.decl.item.signature;
            if signature.identifier.as_str() != name {
                continue;
            }
            let labels = signature.overload_labels();
            let labels = labels
                .iter()
                .map(|label| label.as_str())
                .collect::<Vec<_>>();
            let receiver = signature
                .receiver()
                .and_then(|param| match &param.param_type {
                    TypeElement::Plain(plain) => Some(&plain.ident),
                    TypeElement::Parametric(parametric) => Some(&parametric.base_type),
                    _ => None,
                });
            let id = RustFunctionId::new(receiver, signature.identifier.as_str(), &labels);
            self.by_imported_function.insert(("".to_string(), id), idx);
        }
    }

    fn function_decl(&mut self, crate_name: &str, name: &str, signature: &Value) -> FnDecl {
        let params = signature
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|param| self.param_from_json(crate_name, param))
            .collect::<Vec<_>>();

        let return_type = signature
            .get("output")
            .and_then(|output| self.type_from_json(crate_name, output))
            .unwrap_or_else(TypeElement::void);

        FnSignature {
            visibility: Visibility::public(),
            identifier: Ident::new(name),
            parameters: ParamList {
                params,
                span: Span::default(),
            },
            return_type,
            where_clause: None,
            span: Span::default(),
        }
        .into()
    }

    fn param_from_json(&mut self, crate_name: &str, param: &Value) -> Option<Param> {
        let pair = param.as_array()?;
        let name = pair.first().and_then(Value::as_str).unwrap_or("_");
        let ty = pair.get(1)?;
        let decl_modifier = if type_is_owned(ty) {
            Some(galvan_ast::DeclModifier::Move)
        } else {
            None
        };
        let param_type = self.type_from_json(crate_name, ty)?;

        Some(Param {
            decl_modifier,
            short_name: None,
            identifier: Ident::new(name),
            param_type,
            span: Span::default(),
        })
    }

    fn type_from_json(&mut self, crate_name: &str, ty: &Value) -> Option<TypeElement> {
        if let Some(primitive) = inner_string(ty, "primitive") {
            return Some(primitive_type(primitive));
        }
        if let Some(generic) = inner_string(ty, "generic") {
            return Some(generic_type(generic));
        }
        if let Some(borrowed) = inner(ty, "borrowed_ref") {
            return borrowed
                .get("type")
                .and_then(|inner| self.type_from_json(crate_name, inner));
        }
        if let Some(resolved) = inner(ty, "resolved_path") {
            let name = resolved.get("name").and_then(Value::as_str)?;
            if name == "Result" {
                let args = resolved
                    .get("args")
                    .and_then(|args| inner(args, "angle_bracketed"))
                    .and_then(|args| args.get("args"))
                    .and_then(Value::as_array);
                let success = args
                    .and_then(|args| args.first())
                    .and_then(|arg| inner(arg, "type"))
                    .and_then(|arg| self.type_from_json(crate_name, arg))
                    .unwrap_or_else(TypeElement::infer);
                return Some(TypeElement::Result(Box::new(ResultTypeItem {
                    success,
                    error: Some(plain_type(TypeIdent::new(format!(
                        "{}Error",
                        crate_type_prefix(crate_name)
                    )))),
                    span: Span::default(),
                })));
            }

            self.push_type(crate_name, name);
            return Some(plain_type(TypeIdent::new(name)));
        }
        if let Some(tuple) = inner(ty, "tuple").and_then(Value::as_array) {
            return Some(TypeElement::Tuple(Box::new(galvan_ast::TupleTypeItem {
                elements: tuple
                    .iter()
                    .filter_map(|ty| self.type_from_json(crate_name, ty))
                    .collect(),
                span: Span::default(),
            })));
        }

        Some(TypeElement::infer())
    }

    fn add_curated_crate(&mut self, crate_name: &str) {
        if crate_name != "serde_json" {
            return;
        }

        self.push_type(crate_name, "Error");
        self.push_type(crate_name, "Value");
        self.push_function(
            crate_name,
            "to_string",
            "::serde_json::to_string".into(),
            FnSignature {
                visibility: Visibility::public(),
                identifier: Ident::new("to_string"),
                parameters: ParamList {
                    params: vec![Param {
                        decl_modifier: None,
                        short_name: None,
                        identifier: Ident::new("value"),
                        param_type: generic_type("T"),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                },
                return_type: TypeElement::Result(Box::new(ResultTypeItem {
                    success: plain_type(TypeIdent::new("String")),
                    error: Some(plain_type(TypeIdent::new("Error"))),
                    span: Span::default(),
                })),
                where_clause: None,
                span: Span::default(),
            }
            .into(),
            false,
        );
    }
}

#[derive(Debug)]
pub struct RustTypeDecl {
    pub name: TypeIdent,
    pub rust_path: Box<str>,
    pub decl: ToplevelItem<TypeDecl>,
}

#[derive(Debug)]
pub struct RustFunctionDecl {
    pub namespace: Box<str>,
    pub rust_path: Box<str>,
    pub borrowed_return: bool,
    pub decl: ToplevelItem<FnDecl>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RustFunctionId(Box<str>);

impl RustFunctionId {
    fn new(receiver: Option<&TypeIdent>, name: &str, labels: &[&str]) -> Self {
        let mut id = String::new();
        if let Some(receiver) = receiver {
            id.push_str(receiver.as_str());
            id.push_str("::");
        }
        id.push_str(name);
        if !labels.is_empty() {
            id.push(':');
            id.push_str(&labels.join(":"));
        }
        Self(id.into())
    }
}

struct RustdocCache {
    crate_name: String,
    root: PathBuf,
}

impl RustdocCache {
    fn new(crate_name: &str) -> Self {
        let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            crate_name: crate_name.to_string(),
            root: manifest_dir
                .join("target")
                .join("galvan")
                .join("rustdoc-json"),
        }
    }

    fn json_path(&self) -> Option<PathBuf> {
        let path = self.root.join(format!("{}.json", self.crate_name));
        path.exists().then_some(path)
    }

    fn update_if_needed(&self) {
        if self.json_path().is_some() {
            self.clear_diagnostics();
            return;
        }
        if env::var_os("GALVAN_RUSTDOC_CACHE_UPDATING").is_some() {
            return;
        }

        let _ = fs::create_dir_all(&self.root);
        let manifest_path = match dependency_manifest_path(&self.crate_name) {
            Ok(Some(path)) => path,
            Ok(None) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    format!(
                        "crate '{}' was not found in cargo metadata",
                        self.crate_name
                    ),
                );
                return;
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
                return;
            }
        };

        let target_dir = self.root.join("target");
        let output = Command::new("rustup")
            .arg("run")
            .arg("nightly")
            .arg("cargo")
            .arg("rustdoc")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir)
            .arg("--")
            .arg("-Z")
            .arg("unstable-options")
            .arg("--output-format")
            .arg("json")
            .env("GALVAN_RUSTDOC_CACHE_UPDATING", "1")
            .env_remove("RUSTC")
            .env_remove("RUSTDOC")
            .env_remove("RUSTC_WRAPPER")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let generated = target_dir
                    .join("doc")
                    .join(format!("{}.json", self.crate_name));
                let cached = self.root.join(format!("{}.json", self.crate_name));
                if fs::copy(&generated, &cached).is_ok() {
                    self.clear_diagnostics();
                } else {
                    let _ = fs::write(
                        self.root.join(format!("{}.stderr", self.crate_name)),
                        format!(
                            "rustdoc succeeded but {} was not found\n{}",
                            generated.display(),
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    );
                }
            }
            Ok(output) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    String::from_utf8_lossy(&output.stderr).as_ref(),
                );
                let _ = fs::write(
                    self.root.join(format!("{}.stdout", self.crate_name)),
                    String::from_utf8_lossy(&output.stdout).as_ref(),
                );
            }
            Err(error) => {
                let _ = fs::write(
                    self.root.join(format!("{}.stderr", self.crate_name)),
                    error.to_string(),
                );
            }
        }
    }

    fn clear_diagnostics(&self) {
        let _ = fs::remove_file(self.root.join(format!("{}.stderr", self.crate_name)));
        let _ = fs::remove_file(self.root.join(format!("{}.stdout", self.crate_name)));
    }
}

fn dependency_manifest_path(crate_name: &str) -> Result<Option<PathBuf>, RustdocError> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("RUSTC_WRAPPER")
        .output()
        .map_err(RustdocError::CargoMetadata)?;

    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(RustdocError::InvalidCargoMetadata)?;
    let manifest_path = metadata
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|package| package.get("name").and_then(Value::as_str) == Some(crate_name))
        .and_then(|package| package.get("manifest_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from);

    Ok(manifest_path)
}

fn imported_crates(uses: &[ToplevelItem<UseDecl>]) -> HashSet<String> {
    uses.iter()
        .filter_map(|use_decl| use_decl.path.segments.first())
        .map(|segment| segment.as_str().to_string())
        .collect()
}

fn inner<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => object.get(key),
        _ => None,
    }
}

fn inner_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    inner(value, key).and_then(Value::as_str)
}

fn is_public(item: &Value) -> bool {
    item.get("visibility")
        .is_some_and(|visibility| match visibility {
            Value::String(value) => value == "public",
            Value::Object(object) => object.contains_key("public"),
            _ => false,
        })
}

fn public_type_name(item: &Value) -> Option<&str> {
    let name = item.get("name").and_then(Value::as_str)?;
    let inner = item.get("inner")?;
    ["struct", "enum", "type_alias", "union"]
        .iter()
        .any(|kind| inner.get(*kind).is_some())
        .then_some(name)
}

fn rust_path(crate_name: &str, name: &str, item: &Value) -> Box<str> {
    item.get("path")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("::")
        })
        .filter(|path| !path.is_empty())
        .map(|path| format!("::{path}").into())
        .unwrap_or_else(|| format!("::{crate_name}::{name}").into())
}

fn return_is_borrowed(signature: &Value) -> bool {
    signature
        .get("output")
        .is_some_and(|output| inner(output, "borrowed_ref").is_some())
}

fn type_is_owned(ty: &Value) -> bool {
    inner(ty, "borrowed_ref").is_none()
}

fn plain_type(ident: TypeIdent) -> TypeElement {
    TypeElement::Plain(BasicTypeItem {
        ident,
        span: Span::default(),
    })
}

fn generic_type(name: &str) -> TypeElement {
    TypeElement::Generic(galvan_ast::GenericTypeItem {
        ident: Ident::new(name),
        span: Span::default(),
    })
}

fn primitive_type(name: &str) -> TypeElement {
    let galvan = match name {
        "bool" => "Bool",
        "i8" => "I8",
        "i16" => "I16",
        "i32" => "I32",
        "i64" => "I64",
        "i128" => "I128",
        "isize" => "ISize",
        "u8" => "U8",
        "u16" => "U16",
        "u32" => "U32",
        "u64" => "U64",
        "u128" => "U128",
        "usize" => "USize",
        "f32" => "Float",
        "f64" => "Double",
        "char" => "Char",
        "str" => "String",
        _ => "__UnknownRustPrimitive",
    };
    plain_type(TypeIdent::new(galvan))
}

fn crate_type_prefix(crate_name: &str) -> String {
    crate_name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(name: &str) -> Ident {
        Ident::new(name)
    }

    fn use_decl(segments: &[&str]) -> ToplevelItem<UseDecl> {
        ToplevelItem {
            item: UseDecl {
                path: galvan_ast::UsePath {
                    segments: segments
                        .iter()
                        .map(|segment| Ident::new(*segment))
                        .collect(),
                    span: Span::default(),
                },
                span: Span::default(),
            },
            source: Source::Builtin,
        }
    }

    #[test]
    fn loading_a_crate_does_not_import_its_functions_unqualified() {
        let interop = RustInterop::from_crates_and_uses(["serde_json".to_string()], &[]).unwrap();

        assert!(interop
            .function(Some("serde_json"), None, &ident("to_string"), &[])
            .is_some());
        assert!(interop
            .function(None, None, &ident("to_string"), &[])
            .is_none());
    }

    #[test]
    fn use_declarations_import_functions_unqualified() {
        let uses = [use_decl(&["serde_json"])];
        let interop = RustInterop::from_crates_and_uses([], &uses).unwrap();

        assert!(interop
            .function(None, None, &ident("to_string"), &[])
            .is_some());
    }

    #[test]
    fn path_use_declarations_import_only_the_named_item() {
        let uses = [use_decl(&["serde_json", "to_string"])];
        let interop = RustInterop::from_crates_and_uses([], &uses).unwrap();

        assert!(interop
            .function(None, None, &ident("to_string"), &[])
            .is_some());
        assert!(interop
            .function(None, None, &ident("from_str"), &[])
            .is_none());
    }
}

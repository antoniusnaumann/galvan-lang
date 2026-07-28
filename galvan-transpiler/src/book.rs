use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

use galvan_files::Source;
use galvan_transpiler::{transpile_sources_with_diagnostics_and_rustdoc_warnings, TranspileOutput};

const DEFAULT_BOOK_SOURCE: &str = "documentation/galvan-by-example/src";

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut write = false;
    let mut root = PathBuf::from(DEFAULT_BOOK_SOURCE);
    for argument in env::args().skip(1) {
        match argument.as_str() {
            "--write" => write = true,
            "--check" => write = false,
            _ if argument.starts_with('-') => {
                return Err(format!("unknown option: {argument}"));
            }
            _ => root = PathBuf::from(argument),
        }
    }

    let pattern = Regex::new(
        r"(?s)```galvan[ \t]*\r?\n(?P<galvan>[^`]*)\r?\n```[ \t]*\r?\n\r?\n(?:(?P<aspirational><!-- galvan-book: aspirational -->)[ \t]*\r?\n\r?\n)?<details>[ \t]*\r?\n<summary>Generated Rust</summary>[ \t]*\r?\n\r?\n```rust[ \t]*\r?\n(?P<rust>[^`]*)\r?\n```",
    )
    .map_err(|error| error.to_string())?;

    let mut files = markdown_files(&root)?;
    files.sort();
    let mut checked = 0;
    let mut changed = Vec::new();
    let mut failures = Vec::new();
    let mut compile_examples = Vec::new();

    for path in files {
        let original = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let mut replacements = Vec::new();

        for captures in pattern.captures_iter(&original) {
            checked += 1;
            let galvan = captures
                .name("galvan")
                .expect("Galvan capture is required")
                .as_str();
            let expected = captures.name("rust").expect("Rust capture is required");
            eprintln!("checking {}", path.display());
            let example = match transpile_example(galvan) {
                Ok(actual) => actual,
                Err(error) => {
                    failures.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };
            let actual = example.display;
            if captures.name("aspirational").is_none() {
                compile_examples.push(CompileExample {
                    source: path.clone(),
                    outputs: example.outputs,
                });
            }

            if normalized(expected.as_str()) != normalized(&actual) {
                changed.push(path.clone());
                replacements.push((expected.range(), actual));
            }
        }

        if write && !replacements.is_empty() {
            let mut updated = original;
            for (range, replacement) in replacements.into_iter().rev() {
                updated.replace_range(range, &replacement);
            }
            fs::write(&path, updated)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
    }

    if let Err(error) = cargo_check_examples(&compile_examples) {
        failures.push(error);
    }

    changed.sort();
    changed.dedup();
    if !failures.is_empty() {
        return Err(format!(
            "{} examples could not be transpiled:\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }
    if changed.is_empty() {
        println!("checked {checked} generated Rust blocks; all are current");
        return Ok(());
    }

    if write {
        println!(
            "updated generated Rust blocks across {} files",
            changed.len()
        );
        return Ok(());
    }

    let paths = changed
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "generated Rust is stale in {} files:\n{paths}\nrun `cargo run -p galvan-transpiler --bin galvan-book -- --write`",
        changed.len()
    ))
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    Ok(files)
}

fn collect_markdown_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = fs::read_dir(path)
        .map_err(|error| format!("failed to read directory {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        collect_markdown_files(&entry.path(), files)?;
    }
    Ok(())
}

struct TranspiledExample {
    display: String,
    outputs: Vec<TranspileOutput>,
}

struct CompileExample {
    source: PathBuf,
    outputs: Vec<TranspileOutput>,
}

fn transpile_example(source: &str) -> Result<TranspiledExample, String> {
    let result = transpile_sources_with_diagnostics_and_rustdoc_warnings(
        vec![Source::from_string(source.to_owned())],
        |_| {},
    )
    .map_err(|error| error.to_string())?;
    if !result.diagnostics.is_empty() {
        let diagnostics = result
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{:?}: {}", diagnostic.severity, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("transpiler emitted diagnostics:\n{diagnostics}"));
    }
    let mut outputs = result.outputs;
    outputs.sort_by(|left, right| {
        left.file_name
            .ends_with("galvan_module.rs")
            .cmp(&right.file_name.ends_with("galvan_module.rs"))
            .then_with(|| left.file_name.cmp(&right.file_name))
    });

    let code = outputs
        .iter()
        .filter_map(trim_output)
        .filter(|output| !output.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let formatted = rustfmt(&code)?;
    Ok(TranspiledExample {
        display: formatted.replace(
            "\n\npub(crate) fn __cli_main() {\n    unreachable!(\"This is not a CLI app.\")\n}",
            "",
        ),
        outputs,
    })
}

fn cargo_check_examples(examples: &[CompileExample]) -> Result<(), String> {
    if examples.is_empty() {
        return Ok(());
    }

    let workspace = TemporaryWorkspace::new()?;
    let galvan_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "galvan-transpiler has no workspace parent".to_owned())?;
    let members = (0..examples.len())
        .map(|index| format!("\"example_{index:03}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        workspace.path.join("Cargo.toml"),
        format!("[workspace]\nmembers = [{members}]\nresolver = \"2\"\n"),
    )
    .map_err(|error| format!("failed to create book-check workspace: {error}"))?;

    let mut source_map = Vec::new();
    for (index, example) in examples.iter().enumerate() {
        let member_name = format!("example_{index:03}");
        source_map.push(format!("{member_name}: {}", example.source.display()));
        let member = workspace.path.join(&member_name);
        let source_dir = member.join("src");
        let module_dir = source_dir.join("galvan_module");
        fs::create_dir_all(&module_dir)
            .map_err(|error| format!("failed to create {member_name}: {error}"))?;
        fs::write(
            member.join("Cargo.toml"),
            format!(
                "[package]\nname = \"galvan-book-{member_name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\ngalvan = {{ path = \"{}\" }}\nclap = {{ version = \"4\", features = [\"derive\"] }}\n",
                toml_path(galvan_root)
            ),
        )
        .map_err(|error| format!("failed to write {member_name}/Cargo.toml: {error}"))?;

        for output in &example.outputs {
            let destination = if output.file_name.as_ref() == "galvan_module.rs" {
                source_dir.join("lib.rs")
            } else {
                module_dir.join(output.file_name.as_ref())
            };
            let content = if output.file_name.as_ref() == "galvan_module.rs" {
                format!(
                    "{}\n#[allow(unused_imports)]\nuse galvan_module::*;\n",
                    output.content
                )
            } else {
                output.content.to_string()
            };
            fs::write(&destination, content).map_err(|error| {
                format!(
                    "failed to write generated file {}: {error}",
                    destination.display()
                )
            })?;
        }
    }

    let output = Command::new("cargo")
        .args([
            "check",
            "--workspace",
            "--all-targets",
            "--offline",
            "--message-format",
            "short",
        ])
        .current_dir(&workspace.path)
        .env(
            "CARGO_TARGET_DIR",
            galvan_root.join("target/galvan-book-conformance"),
        )
        .output()
        .map_err(|error| format!("failed to run cargo check for book examples: {error}"))?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "generated Rust did not compile:\n{}\nexample map:\n{}",
        String::from_utf8_lossy(&output.stderr).trim(),
        source_map.join("\n")
    ))
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

struct TemporaryWorkspace {
    path: PathBuf,
}

impl TemporaryWorkspace {
    fn new() -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
            .as_nanos();
        let path = env::temp_dir().join(format!("galvan-book-{}-{nonce}", std::process::id()));
        fs::create_dir(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for TemporaryWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn trim_output(output: &TranspileOutput) -> Option<String> {
    if output.file_name.as_ref() == "galvan_module.rs" {
        return trim_module(output.content.as_ref());
    }

    let mut lines = output.content.lines();
    let first_item = lines.position(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with("use ")
    })?;
    Some(
        output
            .content
            .lines()
            .skip(first_item)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn trim_module(content: &str) -> Option<String> {
    let marker = "pub(crate) mod galvan_module {";
    let inner = content.split_once(marker)?.1.strip_suffix('}')?;
    let lines = inner.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.contains("const __HAS_CLI_COMMANDS"))?
        + 1;
    let body_lines = lines[start..]
        .iter()
        .filter(|line| {
            let line = line.trim();
            !(line.starts_with("mod ") && line.ends_with(';'))
                && !line.starts_with("pub use self::")
                && line != "use crate::*;"
        })
        .copied()
        .collect::<Vec<_>>();
    let mut retained = Vec::new();
    let mut index = 0;
    while index < body_lines.len() {
        if body_lines[index].trim() == "pub(crate) fn __cli_main() {"
            && body_lines
                .get(index + 1)
                .is_some_and(|line| line.contains("This is not a CLI app."))
            && body_lines
                .get(index + 2)
                .is_some_and(|line| line.trim() == "}")
        {
            index += 3;
            continue;
        }
        retained.push(body_lines[index]);
        index += 1;
    }
    let body = retained.join("\n");
    Some(body.trim().to_owned())
}

fn rustfmt(code: &str) -> Result<String, String> {
    let mut child = Command::new("rustfmt")
        .args(["--emit", "stdout", "--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start rustfmt: {error}"))?;
    child
        .stdin
        .take()
        .expect("piped stdin is available")
        .write_all(code.as_bytes())
        .map_err(|error| format!("failed to send code to rustfmt: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for rustfmt: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn normalized(code: &str) -> String {
    code.replace("\r\n", "\n").trim().to_owned()
}

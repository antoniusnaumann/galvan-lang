# Using Rust Crates

Galvan's superpower is its host ecosystem: **any Rust crate in `Cargo.toml`
is a Galvan dependency.** There are no bindings to write. Galvan reads the
dependency's rustdoc JSON and *lifts* its API into Galvan's type system —
`Option<T>` becomes `T?`, `Result<T, E>` becomes `T!E`, `Vec<T>` becomes
`[T]`, `Arc<Mutex<T>>` becomes `ref T`, and so on.

This chapter shows how to call into crates, how `use` imports work, what the
lifting rules are, and where the current boundaries lie.

> [!NOTE]
> Lifting requires a nightly rustdoc toolchain to generate the dependency's
> JSON documentation; the transpiler detects and reports missing toolchains
> during the build. The precise, up-to-date lifting contract lives in the
> repository at `galvan-rustdoc/LIFTINGS.md`.

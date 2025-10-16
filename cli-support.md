# CLI Subcommand Support Implementation Plan

## Current State Analysis

### What's Already Implemented
1. **Tree-Sitter Grammar**: CLI command syntax is fully implemented
   - `cmd` keyword defined in `tree-sitter-galvan/grammar/keywords.js`
   - Command structure: `cmd name(param_list) { body }` with annotation support
   - Parameter syntax supports short/long names: `n name: String` → `-n/--name`
   - Optional parameters supported: `surname: String?`

2. **Example Code**: Clear specification in `example-projects/cli-app/src/main.galvan`
   - Shows expected syntax and doc comment integration
   - Demonstrates parameter mapping to CLI arguments

### What's Missing  
1. **AST Support**: No `CmdDecl` or CLI-related AST nodes
2. **AST Conversion**: No tree-sitter → AST conversion for `cmd` nodes
3. **Transpilation**: No CLI command → Rust clap code generation

## Implementation Plan

### Phase 1: AST Structure Extension

#### 1.1 Add CLI AST Nodes
**File**: `galvan-ast/src/item/toplevel.rs`
- Add `CmdDecl` struct with:
  - `signature: CmdSignature` 
  - `body: Body`
  - `span: Span`
- Add `CmdSignature` struct with:
  - `identifier: Ident`
  - `parameters: ParamList` (reuse existing)
  - `span: Span`
- Update `RootItem` enum to include `Cmd(CmdDecl)`

#### 1.2 Parameter Enhancement
**File**: `galvan-ast/src/item/fn.rs`
- Extend `Param` struct to support CLI argument syntax:
  - Add `short_name: Option<Ident>` field for short flags like `-n`
  - Add `long_name: Option<Ident>` field for long names like `--name`
  - Update parsing to handle `n name: String` syntax

#### 1.3 Documentation Comment Support
**File**: `galvan-ast/src/item/mod.rs`
- Add `DocComment` struct for `/// comment` annotations
- Link doc comments to commands and parameters for help text generation

### Phase 2: AST Conversion Implementation

#### 2.1 Tree-Sitter to AST Conversion
**File**: `galvan-into-ast/src/items/toplevel.rs`
- Add `ReadCursor` implementation for `CmdDecl`
- Add `ReadCursor` implementation for `CmdSignature`  
- Update `RootItem::read_cursor()` to handle `"cmd"` node type
- Parse parameter syntax: `n name: String` → short_name=`n`, long_name=`name`

#### 2.2 Parameter Parsing Enhancement
**File**: `galvan-into-ast/src/items/toplevel.rs`
- Update `Param::read_cursor()` to parse CLI-specific syntax
- Handle optional parameter detection (`Type?`)

### Phase 3: Transpilation to Clap

#### 3.1 Add Clap Dependency
**File**: `galvan-transpiler/Cargo.toml`
- Add `clap = { version = "4.0", features = ["derive"] }`

#### 3.2 CLI Command Transpilation
**File**: `galvan-transpiler/src/transpile_item/cmd_decl.rs` (new)
- Implement `Transpile` for `CmdDecl`
- Generate clap `#[derive(Parser)]` struct
- Map parameters to clap arguments:
  - `n name: String` → `#[arg(short = 'n', long = "name")]`
  - `surname: String?` → `#[arg(short, long)]` with `Option<String>`
- Include doc comments as help text

#### 3.3 Update Toplevel Transpilation
**File**: `galvan-transpiler/src/transpile_item/toplevel.rs`
- Add `Cmd` to `impl_transpile_variants!` macro
- Update main function generation to include CLI parsing

#### 3.4 Main Function Generation
**File**: `galvan-transpiler/src/transpile_item/toplevel.rs`
- Generate clap-based main function when CLI commands present:
  ```rust
  #[derive(Parser)]
  #[command(name = "program")]
  enum Cli {
      Greet(GreetArgs),
      // ... other commands
  }
  
  fn main() {
      match Cli::parse() {
          Cli::Greet(args) => greet_cmd(args),
          // ... other matches
      }
  }
  ```

### Phase 4: Runtime Integration

#### 4.1 Update Main Macro
**File**: `src/lib.rs`
- Modify `main!()` macro to handle CLI applications
- Generate different main function based on presence of `cmd` declarations

#### 4.2 Add Clap to Workspace
**File**: `Cargo.toml`
- Add clap as workspace dependency for transpiled applications

### Phase 5: Testing & Validation

#### 5.1 Test Cases
**File**: `galvan-test/src/cli.galvan` (new)
- Test basic command parsing  
- Test parameter variations (optional, required)
- Test doc comment integration
- Test short/long name mapping

#### 5.2 Integration Test
- Verify `example-projects/cli-app` compiles and runs correctly
- Test generated help text (`program --help`, `program greet --help`)
- Validate parameter parsing works as expected

## Technical Details

### Parameter Mapping Strategy
```galvan
// Galvan syntax:
cmd greet(n name: String, s surname: String?)

// Generated Rust:
#[derive(Parser)]
struct GreetArgs {
    #[arg(short = 'n', long = "name", help = "First name of the person to greet")]
    name: String,
    #[arg(short = 's', long = "surname", help = "Surname of the person to greet")]  
    surname: Option<String>,
}
```

### Doc Comment Integration
```galvan
/// Greets the user
cmd greet(
    /// First name of the person to greet
    n name: String
)
```
Maps to clap help attributes and command descriptions.

### Error Handling
- Invalid parameter syntax should produce clear compilation errors
- Missing required parameters should be caught by clap at runtime
- Type validation handled by clap's built-in parsing

## Implementation Order
1. AST extensions (Phase 1)
2. AST conversion (Phase 2) 
3. Basic transpilation (Phase 3.1-3.2)
4. Main function generation (Phase 3.3-3.4)
5. Runtime integration (Phase 4)
6. Testing (Phase 5)

This implementation leverages Rust's clap crate to provide robust CLI functionality while maintaining Galvan's simple, declarative syntax for defining commands and arguments.
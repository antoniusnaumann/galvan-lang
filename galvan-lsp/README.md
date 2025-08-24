# Galvan Language Server

This directory contains the Language Server Protocol (LSP) implementation for the Galvan programming language.

## Features

The Galvan LSP server provides comprehensive language support including:

- **Text Document Synchronization**: Full document sync with real-time updates
- **Enhanced Diagnostics**: Both syntax and semantic error reporting using tree-sitter and AST infrastructure
- **Smart Hover Information**: Shows AST node information with syntax highlighting and position details
- **Context-Aware Code Completion**: Intelligent keyword and type suggestions based on cursor context
- **Initialize/Shutdown**: Proper LSP lifecycle management

## Tree-sitter Integration

This LSP implementation makes full use of the tree-sitter-galvan grammar for:

- **Precise Syntax Analysis**: Real-time syntax error detection with accurate positioning
- **Rich Hover Information**: Display AST node types, content, and range information
- **Context-Aware Completions**: Different completions based on syntactic context (function scope, type annotations, etc.)
- **Better Error Messages**: More accurate error reporting using tree-sitter's error recovery

## Usage

### Building

```bash
cargo build -p galvan-lsp --features exec
```

### Running

The LSP server communicates via stdin/stdout using the Language Server Protocol:

```bash
./target/debug/galvan-lsp
```

### Integration with Editors

The server can be integrated with any LSP-compatible editor. For example, in VS Code, you would configure:

```json
{
  "galvan-lsp.serverPath": "/path/to/galvan-lsp",
  "galvan-lsp.args": []
}
```

## Architecture

- Built using `tower-lsp` for LSP protocol handling
- Integrates with existing `galvan-ast` and `galvan-into-ast` infrastructure
- Uses `galvan-parse` for tree-sitter-powered syntax analysis
- Provides real-time document parsing and validation
- Designed to be extensible for additional language features

## Technical Details

- **Two-layer Validation**: First tree-sitter for syntax, then AST parsing for semantics
- **Accurate Error Positioning**: Uses tree-sitter's position information for precise error highlighting
- **Efficient Document Handling**: In-memory document storage with incremental updates
- **Comprehensive Testing**: Unit tests for all major functionality

## Future Enhancements

- Semantic analysis and type checking
- Go-to-definition and find references
- Code formatting and refactoring
- Advanced context-aware completions
- Symbol resolution and workspace support
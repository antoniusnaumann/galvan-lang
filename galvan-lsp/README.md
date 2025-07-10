# Galvan Language Server

This directory contains the Language Server Protocol (LSP) implementation for the Galvan programming language.

## Features

The Galvan LSP server provides basic language support including:

- **Text Document Synchronization**: Full document sync with real-time updates
- **Diagnostics**: Basic syntax error reporting using the Galvan AST parser
- **Hover Information**: Shows line content and position information
- **Code Completion**: Basic keyword and type completion for Galvan syntax
- **Initialize/Shutdown**: Proper LSP lifecycle management

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
- Uses existing Galvan AST infrastructure (`galvan-ast`, `galvan-into-ast`)
- Provides real-time document parsing and validation
- Designed to be extensible for additional language features

## Current Limitations

- Tree-sitter grammar is not yet implemented (parsing currently returns placeholder errors)
- Limited to basic LSP features
- No semantic analysis or advanced language features yet

## Future Enhancements

- Complete tree-sitter grammar implementation
- Semantic analysis and type checking
- Go-to-definition and find references
- Code formatting and refactoring
- More sophisticated completions based on context
# Galvan Language TODOs

## Critical - Core Language Features
- **Type inference system improvements** (galvan-transpiler/src/type_inference.rs)
  - Add support for type unions (line 132)  
  - Implement function lookup with borrowed return values (line 402)
  - Handle inference for alias types (lines 813, 891)

- **Missing operator implementations** (galvan-transpiler/src/transpile_item/operator.rs)
  - Implement XOR chain handling (line 21)
  - Add remove operator for collections (line 165)
  - Implement custom infix operators (line 195)

- **Generic type support** (galvan-transpiler/src/transpile_item/type.rs)
  - Transpile generic type parameters (line 42)
  - Handle generics in type elements (galvan-transpiler/src/lib.rs line 412)

## High Priority - Language Completeness
- **Assignment handling improvements** (galvan-transpiler/src/transpile_item/assignment.rs)
  - Determine variable ownership for proper dereferencing (line 12)
  - Handle assignment to ref variables (line 37)
  - Add proper error handling for borrowed variable assignment (line 32)

- **Collection operations** (galvan-transpiler/src/transpile_item/collection.rs)
  - Implement OrderedDictLiteral transpilation (line 48)

- **Function call enhancements** (galvan-transpiler/src/transpile_item/function_call.rs)
  - Add capacity optimization for vector creation (line 512)
  - Implement for loop on optional types (line 425)
  - Add tuple iteration support (line 421)

## Medium Priority - Error Handling & Validation
- **Comprehensive error handling** (multiple files)
  - Replace todo!() calls with proper Result types throughout codebase
  - Add validation for function modifiers, closure arguments, type assertions

- **Type validation** 
  - Add type checking for collection operations (postfix.rs line 9)
  - Validate error/optional type usage (postfix.rs line 16-17)
  - Check struct field modifier validity (struct.rs line 23, 26)

## Low Priority - Language Polish
- **String formatting** (galvan-transpiler/src/transpile_item/literal.rs)
  - Add number literal parsing and type validation (line 24)

- **Identifier improvements** (galvan-transpiler/src/transpile_item/ident.rs)  
  - Implement fully qualified name lookup (line 10, 24)

- **Tree-sitter grammar completeness** (tree-sitter-galvan/)
  - Add const/async keyword support
  - Replace annotation placeholder with actual implementation
  - Add implicit closure parameter rules

## Future Enhancements
- Add "todo" and "panic" as special handling functions
- Implement build entry points and custom tasks (galvan-into-ast/src/items/toplevel.rs)
- Add nested contexts for imported module name resolution (galvan-resolver/src/lookup.rs)
- Improve span tracking throughout AST nodes

---
*Last updated: 2025-10-08*
*This file should be updated regularly as TODOs are completed or new ones are discovered*
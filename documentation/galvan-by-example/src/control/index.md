# Control Flow

Galvan's control flow constructs are expressions wherever that makes sense:
`if` yields a value, `match` returns its branch results, and even `for` can
collect into an array. Blocks use braces, conditions need no parentheses.

> [!WARNING]
> A general `loop { ... }` construct is designed but **not fully implemented
> yet** — use `for` with a range or collection in the meantime.

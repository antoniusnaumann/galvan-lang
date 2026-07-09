# Collections

Galvan builds its four collection types into the syntax. Brackets indicate
order; braces indicate hashing:

| Syntax | Collection | Rust backing type |
| --- | --- | --- |
| `[T]` | array | `Vec<T>` |
| `{T}` | set | `HashSet<T>` |
| `{K: V}` | dictionary | `HashMap<K, V>` |
| `[K: V]` | ordered dictionary | `IndexMap<K, V>` *(currently `BTreeMap`)* |

Literals mirror the type syntax: `[1, 2, 3]`, `{"a", "b"}`, `{"a": 1}`,
`["a": 1]`. The same shapes appear in type positions: `[Int]`, `{String}`,
`{String: Int}`, `[String: Int]`.

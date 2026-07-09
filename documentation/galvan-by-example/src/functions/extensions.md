# Extension Functions

Because any function with a `self` parameter is a method, you can add methods
to types you don't own — including builtins and collection types:

```galvan
fn shouted(self: String) -> String {
    self.to_uppercase() ++ "!"
}

fn counted(self: [Int], value: Int) -> USize {
    self.iter()
        .copied()
        .filter |it| { it == value }
        .count()
}

fn main() {
    println("galvan".shouted())

    let numbers = [1, 2, 3, 2, 1]
    println("\(numbers.counted(2))")
}
```

Within the declaring crate, extension methods are available everywhere.
Other crates reach them through namespace-qualified calls or a `use` import —
see [Methods and Associated Items](../interop/methods.md).

<details>
<summary>Generated Rust</summary>

```rust
pub trait String_Ext {
    fn shouted(&self) -> String;
}

impl String_Ext for String {
    fn shouted(&self) -> String {
        [(self.to_uppercase()).to_owned(), (format!("!")).to_owned()].concat()
    }
}

pub trait Array_Int_Ext {
    fn counted(&self, value: i64) -> usize;
}

impl Array_Int_Ext for ::std::vec::Vec<i64> {
    fn counted(&self, value: i64) -> usize {
        self.iter().copied().filter(|it| (it).eq(&value)).count()
    }
}

pub(crate) fn __main__() {
    println!("{}", &(&format!("galvan")).shouted());
    let numbers: ::std::vec::Vec<_> = vec![1, 2, 3, 2, 1];
    println!("{}", &format!("{}", (&numbers).counted(2)));
}
```

Extending a foreign type generates an extension trait plus an `impl` for the
target type — the standard Rust pattern, written for you.

</details>

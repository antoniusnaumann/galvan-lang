# Galvan

[![Tests](https://github.com/antoniusnaumann/galvan-lang/actions/workflows/tests.yaml/badge.svg)](https://github.com/antoniusnaumann/galvan-lang/actions/workflows/tests.yaml)
[![Examples](https://github.com/antoniusnaumann/galvan-lang/actions/workflows/examples.yaml/badge.svg)](https://github.com/antoniusnaumann/galvan-lang/actions/workflows/examples.yaml)

A high-level application language with concise syntax, value-oriented defaults,
and explicit escape hatches when shared mutable state is needed.

> [!IMPORTANT]
> Galvan is a work in progress and still a hobby project. The core examples in
> this README reflect implemented behavior unless a section has an explicit
> implementation warning. If you like the ideas here and want to help, feel free
> to contact me or start a discussion on GitHub.

> [!NOTE]
> Galvan is a companion language that transpiles to Rust. It is not intended to
> replace Rust for low-level systems work; it is aimed at application code such
> as CLI tools, services, and other high-level programs that still benefit from
> the Rust ecosystem.

## Motivation

Galvan focuses on the parts of day-to-day application programming where a
language can provide strong defaults: concise declarations, ergonomic
collections, predictable ownership behavior, and integrated tooling for tests
and command-line interfaces.

Galvan is not intended for low-level programming. You should not use it to write
a parser, compiler, allocator, or audio compression library. It is designed for
code where readability and fast iteration matter more than controlling every
implementation detail.

> [!NOTE]
> Rust is excellent when explicit control over memory, lifetimes, and low-level
> performance is the central requirement. Galvan keeps Rust interoperability as a
> goal while moving common application patterns into simpler syntax.

## A Tour Of Galvan

### Programs And Strings

Galvan programs use a regular `main` function:

```galvan
fn main() {
    let name = "Galvan"
    print("Welcome to \(name)!")
}
```

The main function can optionally receive the process argument vector. The first
element is the executable name:

```galvan
fn main(args: [String]) {
    print args
}
```

Async functions use `async fn` and await futures with `.await`. Functions that
use postfix `!` still return a result type:

```galvan
async fn main() -> ! {
    let response = client.get("https://example.com").send().await!
    print response.text().await!
}
```

An async `main` function uses the default async runtime; it does not need a
runtime attribute in Galvan code.

> [!NOTE]
> In the Rust target, Galvan's default async runtime is Tokio.

> [!WARNING]
> Async functions, `.await`, and async `main` generation are not implemented
> yet.

Strings support inline interpolation:

```galvan
let count = 3
print("Packed \(count) orders")
```

Interpolation can contain member access and expressions. Strings also support
escaped quotes, literal braces, and Unicode escapes:

```galvan
let dog = Dog(name: "Milo")

print("Hi, \(dog.name)!")
print("3 + 7 = \(3 + 7)")
print("{\(3 + 7)}")
print("\u{1F600}")
```

Character literals use single quotes and support common escape sequences:

```galvan
let first = 'a'
let newline = '\n'
let quote = '\''
let greeting = "hello" ++ '!'
```

### Functions

Functions are declared with `fn`. A function returns the value of its final
expression unless it returns early:

```galvan
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

### Types

Types are declared with the `type` keyword:

```galvan
pub type Color {
    r: Int
    g: Int
    b: Int
}

pub type Person(name: String, age: Int)
pub type Human = Person
pub type Couple(Person, Person)
```

Enums use the same `type` keyword. Variants can have no values, tuple-like
values, or named values:

```galvan
pub type Theme(name: String) {
    Plain
    Monochrome(Color)
    Dark(background: Color, foreground: Color)
    Light(background: Color, foreground: Color)
}
```

The `name` field in `Theme(name: String)` is common data shared by all variants.
Variant-specific fields are declared on each case.

Enum variants are constructed and referenced with `Type::Variant` syntax:

```galvan
pub type ColorChoice {
    Transparent
    Gray(U8)
    Rgb(r: U8, g: U8, b: U8)
}

let transparent = ColorChoice::Transparent
let gray = ColorChoice::Gray(128)
let rgb = ColorChoice::Rgb(r: 100, g: 10, b: 150)
```

Struct fields can provide default values:

```galvan
type Book {
    title: String = "Field Notes"
    content: String = "No notes yet"
}

fn main() {
    let book = Book()
}
```

> [!NOTE]
> When a type can be constructed without arguments, Galvan automatically emits a
> Rust `Default` implementation for the generated type.

### Auto Traits

Some traits are auto traits in Galvan. A type conforms automatically when all
of its fields conform, unless the type opts out:

```galvan
@derive(!Clone)
type SessionToken {
    value: String
}
```

The built-in auto traits are `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`,
`Eq`, `Hash`, `serde::Serialize`, and `serde::Deserialize`.

Explicit `@derive(...)` can still be written when a type wants to be clear
about intended conformance:

```galvan
@derive(Clone, Debug, serde::Serialize)
type HealthResponse {
    status: String
}
```

An explicit trait implementation can override the derived behavior. Libraries
can declare additional auto traits with `auto trait`:

```galvan
auto trait CacheSafe
```

> [!WARNING]
> The full auto-trait model, `@derive(...)` annotations, opt-outs such as
> `@derive(!Clone)`, and user-declared `auto trait`s are not implemented yet.

### Member Functions

All functions are declared top-level. If the first parameter is named `self`,
the function can be called as a member function:

```galvan
pub type Dog { name: String }

fn bark(self: Dog) {
    print("\(self.name) barks")
}

fn main() {
    let dog = Dog(name: "Milo")
    dog.bark()
}
```

### Namespaces

Items in the same crate are available unqualified. Items from dependency crates
are automatically available through namespace-qualified syntax such as
`mycrate::my_item()`.

> [!NOTE]
> `use mycrate` imports all public items from that crate for unqualified use,
> similar to `use mycrate::*` in Rust. Path imports such as
> `use mycrate::my_item` import only that item for unqualified use.

Associated functions and constants on types use member syntax. `::` selects the
namespace portion of a path; methods and constants that belong to the final type
use `.`:

```galvan
let addr = std::net::SocketAddr.from(([127, 0, 0, 1], 3000))
let created = axum::http::StatusCode.CREATED
```

Galvan also allows methods to be added to types you do not own. Outside the
defining crate, namespace-qualified calls are available without an import. Use
the namespace only when you want the methods available unqualified:

```galvan
fn score_book() {
    let book = Book()
    let score = book.reader::read_and_judge()
}

use reader

fn score_book_after_import() {
    let book = Book()
    let score = book.read_and_judge()
}
```

If method names from two imported crates clash, use qualified syntax.

### Overloading

Limited overloading is supported through argument labels:

```galvan
fn pick(value: U8) -> U8 {
    value
}

fn pick(value: U8, plus increment: U8) -> U8 {
    value + increment
}

fn pick(value: U8, plus increment: U8, ~ fallback: U8) -> U8 {
    value + increment + fallback
}

fn main() {
    assert pick(1) == 1
    assert pick(2, plus: 3) == 5
    assert pick(4, plus: 5, fallback: 6) == 15
}
```

The `~` marker means the call label should be the same as the parameter name.

> [!NOTE]
> Generated Rust function names are label-mangled, such as `pick`,
> `pick__plus`, and `pick__plus__fallback`. Galvan names forbid double
> underscores to avoid clashes with generated names.

## Data And Ownership

### Collections

Galvan has concise syntax for common collection types:

```galvan
pub type IntArray = [Int]
pub type StringSet = {String}
pub type Inventory = {String: Int}
pub type DailyMenu = [String: Int]
```

Ordered collection types use `[]`; unordered collection types use `{}`.

> [!NOTE]
> These collection forms map to common Rust collection types: arrays are backed
> by `Vec`, sets by `HashSet`, dictionaries by `HashMap`, and ordered
> dictionaries by `IndexMap`.

### Pass-By-Value, `mut`, And `ref`

Arguments are passed by value by default. If a function needs to mutate the
caller-owned value, mark the parameter as `mut`:

```galvan
fn make_uppercase(mut name: String) {
    name = name.to_uppercase()
}

fn shouted(name: String) -> String {
    name.to_uppercase()
}
```

When calling a function with a `mut` parameter, the caller must annotate the
argument:

```galvan
fn main() {
    mut name = "milo"
    make_uppercase(mut name)
}
```

Argument modifiers can also be written in postfix form:

```galvan
fn main() {
    mut name = "milo"
    make_uppercase(name.mut)
}
```

The same explicit modifiers are required when the receiver of a member function
is declared as `mut self` or `ref self`:

```galvan
fn rename(mut self: Dog, name: String) {
    self.name = name
}

fn replace(ref self: Dog, replacement: Dog) {
    self = replacement
}

fn main() {
    mut dog = Dog(name: "Milo")
    dog.mut.rename("Scout")

    ref shared_dog = Dog(name: "Rex")
    shared_dog.ref.replace(Dog(name: "Lassie"))
}
```

Stored references are declared with `ref`. They use reference semantics and can
be shared through structs or variables:

```galvan
pub type Person {
    name: String
    age: Int
    ref dog: Dog
}

fn main() {
    ref dog = Dog(name: "Milo", age: 5)
    let person = Person(name: "Jochen", age: 67, dog: ref dog)

    dog.age += 1

    print(person.dog.age) // 6
    print(dog.age) // 6
}
```

Passing a `ref` value into a `ref` parameter also requires an explicit call-site
modifier:

```galvan
fn store(ref value: String) {
    // ...
}

fn main() {
    ref label = "shared"
    store(ref label)
    store(label.ref)
}
```

> [!NOTE]
> Galvan's `mut value: T` is generated as a mutable Rust reference. Galvan does
> not expose immutable references directly; immutable values are treated with
> copy-on-write-style defaults, while `ref` values use heap-backed reference
> semantics.

`ref` variables can also be reassigned by reference:

```galvan
fn main() {
    ref message = "Hello"
    ref alias = ref message

    message = "Hi"
    print alias // "Hi"

    ref farewell = "Bye"
    alias = ref farewell

    print message // "Hi"
    print alias // "Bye"
}
```

## Optionals, Results, And Control Flow

### Optionals And Results

Galvan uses `?` for optional types and `!` for result types:

```galvan
type OptionalInt = Int?
type FileOrFlexibleError = File!
type FileOrIoError = File!IoError
```

The error type is written after `!`. If it is omitted, Galvan uses a flexible
error type.

The postfix `!` operator unwraps a result and returns early if it contains an
error:

```galvan
fn read_title(path: String) -> String! {
    let file = File.open(path)!
    file.read_to_string()!
}
```

> [!NOTE]
> Galvan's postfix `!` operator corresponds to Rust's `?` operator for early
> error return. Galvan uses `?` for safe calls instead.

The safe-call operator `?.` continues only when the receiver is not `none` and
not an error. `else` can provide a fallback for optionals and results:

```galvan
let displayed_name = maybe_user?.name else { "Anonymous" }
let points = load_reward_points() else { 0 }
```

Values are automatically wrapped when an optional or result is expected:

```galvan
let selected_count: Int? = 5
let loaded_count: Int!String = 7

fn count_or_default(count: Int?) -> Int {
    count else { 0 }
}

assert count_or_default(21) == 21
```

### If And Else

`if` can be used as a statement or as an expression:

```galvan
let shipping_status = if paid {
    "ready"
} else if inventory_reserved {
    "waiting for pickup"
} else {
    "blocked"
}
```

An `if` expression without an `else` produces an optional value:

```galvan
let discount = if customer_is_member { 10 }
```

### Match

`match` expressions can return branch values and destructure enum variants:

```galvan
fn classify_color(color: Color) -> String {
    match color {
        Transparent { "transparent" }
        Gray(value) { "gray \(value)" }
        Rgb(r: red, b: blue, g: _) { "rgb \(red) \(blue)" }
        _ { "unknown" }
    }
}
```

### Try

`try` unwraps an optional or result. The unwrapped value is available through
`it`, or through an explicitly named binding:

```galvan
try load_reward_points() {
    print("Loaded \(it) points")
} else {
    print("Could not load points: \(it)")
}

try maybe_user |user| {
    print(user.name)
} else {
    print("No user selected")
}
```

Like `if`, `try` can also be used without an `else` branch:

```galvan
try maybe_user |user| {
    print(user.name)
}

let selected_name = try maybe_user |user| { user.name }
```

### For Loops

For loops work over ranges, collections, optionals, and results:

```galvan
for 0..<n {
    print(it)
}

for 0..<n |index| {
    print(index)
}
```

For loops can also be expressions. In expression position, they collect each
iteration result into an array:

```galvan
let doubled_even: [Int] = for 0..=n {
    if it % 2 == 1 { continue }
    it * 2
}
```

Range bounds use `..<` for exclusive upper bounds and `..=` for inclusive upper
bounds.

> [!WARNING]
> General `loop { ... }` expressions are not implemented yet. `for` loops are
> implemented for arrays, sets, dictionaries, ordered dictionaries, optionals,
> results, and ranges. Tuple iteration is still incomplete.

### Return And Throw

Return values are implicit, but `return` can be used for early returns:

```galvan
fn fib(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    fib(n - 1) + fib(n - 2)
}
```

Errors are returned early with `throw`:

```galvan
fn checked_divide(a: Float, b: Float) -> Float! {
    if b == 0 {
        throw "Division by zero"
    }

    a / b
}
```

## Abstraction Features

### Generics

Type identifiers start with an uppercase letter. A lowercase type name
introduces a type parameter:

```galvan
type Container {
    value: t
}

fn get_value(self: Container<t>) -> t {
    self.value
}
```

Bounds can be specified with `where`:

```galvan
fn concat_text(self: t, other: t) -> String where t: ToString {
    self.to_string() ++ other.to_string()
}
```

> [!WARNING]
> Generic syntax, basic generic functions, methods, and `where` clauses are
> implemented, but generic type inference and compatibility checking are still
> incomplete. Some generic cases may produce warnings or fall back to broad
> compatibility.

### Closures

Closures use parameter-list syntax:

```galvan
let add = |a, b| a + b
```

Closure types use the same `|_|` shape:

```galvan
fn map(self: [t], f: |t| u) -> [u] {
    mut result = []
    for self {
        result.push(f(it))
    }
    result
}
```

Functions with trailing closures can omit the regular argument parentheses:

```galvan
orders
    .map |order| { order.total }
    .filter |total| { total > 0 }
    .fold 0 |acc, total| { acc + total }
```

> [!WARNING]
> Numbered closure parameters such as `#0` and `#1` are planned but not
> implemented yet.

### Parentheses-Free Function Calls

In a statement or on the right-hand side of an assignment, function-call
parentheses can be omitted:

```galvan
fn add(a: Int, b: Int) -> Int {
    a + b
}

fn main() {
    let result = add 2, 3
    print result
}
```

This syntax is not allowed in function arguments, and calls with no arguments
still require parentheses to avoid ambiguity with variables.

## Operators

### Builtin Operators

Arithmetic operators:

- `+`: Addition
- `-`: Subtraction
- `*`: Multiplication
- `/`: Division
- `%`: Remainder
- `^`: Exponentiation

Logical operators:

- `and`, `&&`: Logical and
- `or`, `||`: Logical or
- `xor`: Logical xor
- `not`, `!`: Logical not

> [!NOTE]
> Galvan does not provide Unicode alternatives for logical operators because
> `∧` and `∨` can be confused with `v` and `^`. Custom operators can provide
> those spellings later.

Bitwise operators:

- `|`: Bitwise or
- `&`: Bitwise and
- `~`: Bitwise xor
- `<<`: Bitwise left shift
- `>>`: Bitwise right shift

Comparison operators:

- `==`: Equality
- `!=`, `≠`: Inequality
- `<`: Less than
- `<=`, `≤`: Less than or equal
- `>`: Greater than
- `>=`, `≥`: Greater than or equal
- `===`, `≡`: Pointer equality for heap references
- `!==`, `≢`: Pointer inequality for heap references

Collection operators:

- `++`: Concatenation
- `--`: Removal
- `**`: Repetition
- `[]`: Indexing
- `[:]`: Slicing
- `in`, `∈`, `∊`: Membership

Range operators:

- `..<`: Exclusive range
- `..=`: Inclusive range
- `+-`, `±`: Inclusive range around a value

> [!WARNING]
> Some listed operators are still incomplete. In particular, collection removal
> (`--`), repetition (`**`), slicing (`[:]`), unary logical not, and custom
> operators need additional implementation work.

### Canonical Operator Implementation

Galvan's intended operator model is structural: operators should be derived for
types whose members support the same operation.

```galvan
type Vec2 {
    x: Float
    y: Float
}

test "Automatically derive addition for struct" {
    let this_vec = Vec2(x: 5.0, y: 10.0)
    let that_vec = Vec2(x: 7.0, y: 1.0)

    let result = this_vec + that_vec

    assert result.x == this_vec.x + that_vec.x
    assert result.y == this_vec.y + that_vec.y
}
```

> [!WARNING]
> Canonical operator implementation is not implemented yet.

### Union Types

Galvan's intended union syntax uses `|` where a type identifier is expected:

```galvan
fn print_value(value: Int | String) {
    print("Value: \(value)")
}
```

> [!WARNING]
> Union types are not implemented yet.

## Semicolon Inference

Galvan uses semicolons to separate statements, but infers semicolons on newlines
when:

- the next line starts with an alpha character or underscore as the first
  non-whitespace character
- the next line starts with `{`, `(`, `[`, `'`, or `"` as the first
  non-whitespace character

Galvan does not infer a semicolon when the current line itself is not a valid
statement. It also infers commas for struct type declarations when fields are
separated by newlines.

## Tooling-Oriented Features

### Testing

Galvan provides a concise syntax for unit tests in any `.galvan` file:

```galvan
test {
    assert 2 == 2
}

test "Ensure that addition works correctly" {
    assert 2 + 2 == 4
}
```

Test descriptions are optional but encouraged.

### CLI Argument Parsing

Galvan has built-in support for CLI apps with arguments and subcommands:

```galvan
cmd main(
    /// Optional name to greet when no subcommand is selected
    n name: String?
) {
    try name |name| {
        print "Hello \(name)!"
    } else {
        print "Hello World!"
    }
}

/// Greets the user
cmd greet(
    /// First name of the person to greet
    n name: String,
    /// Surname of the person that should be greeted
    s surname: String?
) {
    try surname |surname| {
        print "Hello \(name) \(surname)!"
    } else {
        print "Hello \(name)!"
    }
}
```

> [!NOTE]
> The CLI support is generated through the Rust `clap` crate. Top-level
> `cmd main` arguments become top-level flags, and other `cmd` declarations
> become subcommands.

The generated CLI looks like this:

```text
$ my-app --help
Commands:
  greet  Greets the user
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ my-app greet --help
Greets the user

Usage: my-app greet [OPTIONS] --name <NAME>

Options:
  -n, --name <NAME>        First name of the person to greet
  -s, --surname <SURNAME>  Surname of the person that should be greeted
  -h, --help               Print help
```

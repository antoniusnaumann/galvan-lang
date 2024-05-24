# Galvan
A high-level companion language for Rust.

> [!IMPORTANT]
> This is a work in progress and under development. It currently is in a state of a hobby project and most features described here are not yet implemented.
> I am working on this project in my free time - if you like the ideas presented here, and want to help, feel free to contact me or start a discussion here on GitHub.

## Motiviation
### What Galvan is not
Galvan is not intended as a replacement for Rust. It is a companion language that transpiles to Rust.
That means, Galvan's abstractions are not always zero-cost, instead Galvan tries to pick a sane default choice for most use cases.

Galvan is not intended for low-level programming - so you should not build a parser, compiler or audio compression library with it.
Instead, Galvan is intended to be used for high-level applications, like CLI tools, web servers and so on. The ultimate goal is full Rust interoperability, so you can write your application in Galvan and rely on the full Rust ecosystem.

### Why Galvan?
Rust is a great language, but it is not always the best choice for every task. Rust's syntax can be quite verbose and its type system and borrow checker - while extremely powerful - can be a burden for high-level applications and overwhelming for beginners.
For low-level libraries and so-called "systems programming", Rust hits the sweet spot between helpful abstractions and being in control of implementation details. For application-level programming however, being provided with sensible choices for common use cases is important.
This is where Galvan comes in: It provides a concise syntax and simplified way of writing Rust - without worrying about lifetimes, ownership and so on.

## A Tour of Galvan
### Introduction to Galvan

Galvan is a modern programming language that transpiles to Rust. It provides a concise syntax while leveraging the full power of Rust's ecosystem.

### Basic Syntax and String Formatting
In Galvan, `main` is not a function but an "entry point".
```rust
main {
    let name = "Galvan";
    print("Welcome to {name}, the modern language!")
}
```
Note that Galvan strings always support inline format arguments.


### Functions
Like in Rust, functions are defined with the `fn` keyword and return the value of the last expression:
```rust
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

Very short functions can also be defined with = and have their return type inferred:
> [!WARNING]
> Defining functions with '=' is not implemented yet
```rust
fn add(a: Int, b: Int) = a + b
```
Those functions are not allowed to have newlines in their body.

### Types 
Types in Galvan are defined with the `type` keyword.
```rust
/// A struct definition
pub type Color {
    r: Int,
    g: Int,
    b: Int
}

// Structs can also use the named tuple syntax
pub type Person(name: String, age: Int)

/// A type alias
pub type Human = Person

/// A tuple type
pub type Couple(Person, Person)
```

Enums can have associated values, either for all variants or for specific variants. Enums are also declared using the `type` keyword:
> [!WARNING]
> Enums are not implemented yet
```rust
/// An enum type
/// Enums can have general fields that are accessible to all enum variants
pub type Theme(name: String) {
    Plain
    /// Like in Rust, enum variants can have associated values, either named or unnamed
    Monochrome(Color)
    /// Unlike in Rust, '(' is also used for enum variants with named fields
    Dark(background: Color, foreground: Color)
    Light(background: Color, foreground: Color)
}
```

### Member Functions
All functions are declared top-level. If their first parameter is named `self`, they can be called as member functions:
```rust
pub type Dog(name: String)

fn bark(self: Dog) {
    print("{self.name} barks")
}

main {
    let dog = Dog(name: "Bello");
    dog.bark()
}
```

### Collections

Galvan features syntactic sugar for collection types:
```rust
pub type IntArray = [Int] // This is a Vec
pub type StringSet = {String} // This is a HashSet
pub type MyDict = {String: Int} // This is a HashMap
pub type OrderedDict = [String: Int] // This is an IndexMap
```

Ordered types use `[]`, unordered types use `{}`.

### Optionals and Result Types
Galvan provides concise syntax for optionals and result types:

```rust
type OptionalInt = Int?
type FileOrErr = File!
type FileOrIoErr = File!IoError
```
The error variant is specified after the `!` symbol. If it is not given, a flexible error type is used.

> [!WARNING]
> `!` and `?` are not implemented yet
```rust
fn open_file(path: String) -> File! {
    let file = File::open(path)!;
    let contents = file.read_to_string()?.find("foo")?.uppercase() else { "" };
    
    contents
}
```
`!` operator unwraps the result and early returns if the result is an error. This is identical to the `?` operator in Rust.

`?` is the safe call operator in Galvan. The subsequent expression is only evaluated if the result is not an error and not none.

`else` can also be used as the null-coalescing operator (since you can use else after every expression that is an optional or a result type), you can use it to provide a default if the left-hand side expression is none. 

### Union Types
Galvan supports union types everywhere where a type identifier is expected:
> [!WARNING]
> Union types are not implemented yet
```rust
fn print_value(value: Int | String) {
    print("Value: {value}")
}
```

### Pass-by-Value and Pass-by-Reference
#### mutable vs. immutable function parameters
By default, arguments are passed by value. If the argument needs to be mutated, the `mut` keyword can be used to pass it by reference:
For consistency, the `let` keyword is allowed as well but redundant as parameters are passed by value by default.
```rust
fn add_one(mut value: Int) {
    value += 1
}

// Using `let` is not necessary here but allowed
fn incremented(let value : Int) -> Int {
    value + 1
} 
```

Galvan's `mut value: T` would be equivalent to Rust's `value: &mut T`. Galvan does not have immutable references, as all values are copy-on-write.
```rust
// No copy is happening here as the value is not mutated
// Arguments are passed by value by default
fn bark_at(self: Dog, other: Dog) {
    print("{self.name} barks at {other.name}")
}
```

```rust
// A copy is happening here as the value is mutated
fn shout_at(self: Dog, other: Dog) {
    // Redeclaring is neccessary as value parameters cannot be mutated
    let other = other;
    // Copy is happening here
    other.name = other.name.uppercase();
    print("{self.name} shouts at {other.name}")
}
```

```rust
fn grow(mut self: Dog) {
    // This mutates the original value as it is passed by reference
    self.age += 1
}
```

#### Stored References
References that are allowed to be stored in structs have to be declared as heap references. This is done by prefixing the declaration with `ref`:
```rust
pub type Person {
    name: String,
    age: Int,
    // This is a heap reference
    ref dog: Dog
}

main {
    // Note that constructors use '(' with named arguments
    ref dog = Dog(name: "Bello", age: 5);
    // The `dog` field now points to the same entity as the `dog` variable 
    let person = Person(name: "Jochen, age: 67, dog: ref dog);
    dog.age += 1;
    
    print(person.dog.age); // 6
    print(dog.age) // 6
}
```
Heap references use atomic reference counting to be auto-freed when no longer needed and are always mutable.
In contrast to `let` and `mut` values, `ref` values. They follow reference semantics, meaning that they point to the same object. For this reason, they are always mutable.

#### Argument Modifiers
When calling a function with `mut` or `ref` parameters, you have to annotate the argument respectively. This is not the case for the receiver of a member function.

```rust
fn make_uppercase(mut arg: String) { ... }

fn store(ref arg: String) { ... }

main {
    ref my_string = "This is a heap ref";
    
    // Argument must be annotated as mutable
    make_uppercase(mut my_string);
    // Argument must be annotated as ref
    store(ref my_string)
}
```
By annotating the argument as `mut`, the caller acknowledges that the given argument might be mutated in-place when calling this function.
Immutable variables or members of immutable struct instances (declared with `let`) cannot be passed as `mut`. 

By annotating the argument as `ref`, the caller acknowledges that the function might store a mutable (heap) reference.
Only variables and members declared as `ref` can be passed as `ref`

### Control Flow
#### Loops
Like in Rust, loops can yield a value:
> [!WARNING]
> Loops are not implemented yet
```rust
mut i = 0
let j = loop {
    if i == 15 {
        return i
    }
    i += 1
}
print(j) // 15
print(i) // 15
```

For loops are also supported:
> [!WARNING]
> For loops are not implemented yet
```rust
for 0..<n {
    print(it)
}
```
The loop variable is available via the `it` keyword, but can also be named explicitly using closure parameter syntax:
```rust
for 0..<n |i| {
    print(i)
}
```

Note that ranges are declared using `..<` (exclusive upper bound) or `..=` (inclusive upper bound).

#### If-Else
> [!WARNING]
> Nested if-else is not implemented yet
```rust
if condition {
    print("Condition is true")
} else if other_condition {
    print("Other condition is true")
} else {
    print("No condition is true")
}
```

#### Try
You can use try to unwrap a result or optional:
> [!WARNING]
> Implicit arguments via `it` are not implemented yet
```rust
try potential_error {
    print("Optional was {it}")
} else {
    print("Error occured: {it}")
}
```
The unwrapped variant is available via the it keyword, like in closures. You can also name it using closure parameter syntax to declare them explicitly:
```rust
try potential_error |value| {
    print("Optional was {value}")
} else |error| {
    print("Error occured: {error}")
}
```
Like `if`, you can also use `try` without an else branch:
```rust
try potential_error |value| {
    print("Optional was {value}")
}

let optional = try potential_error |successful| { successful }
```
This can be useful to pass the unwrapped value to a function, or to convert a Result to an Optional.

#### Return and Throw
Return values are implicit, however you can use the `return` keyword to return early:
> [!WARNING]
> Return keyword is not implemented yet
```rust 
fn fib(n: Int) -> Int {
    if n <= 1 {
        return n
    }
    fib(n - 1) + fib(n - 2)
}
```

Returning an error early is done using the `throw` keyword:
> [!WARNING]
> Throw keyword is not implemented yet
```rust
fn checked_divide(a: Float, b: Float) -> Float! {
    if b == 0 {
        throw "Division by zero"
    }
    a / b
}
```

### Generics
In Galvan, type identifiers are always starting with an upper case letter. Using a lower case letter instead introduces a type parameter:
> [!WARNING]
> Generics are not implemented yet
```rust
type Container {
    value: t
}

fn get_value(self: Container<t>) -> t {
    self.value
}
```

Bounds can be specified using the `where` keyword:
```rust
fn concat_hash(self: t, other: t) -> t where t: Hash {
    self.hash() ++ other.hash()
}
```

### Operators
#### Builtin Operators
Galvan offers a wide range of builtin operators. While all of them have an ASCII variant, Galvan also accepts a unicode symbol where it makes sense.

Arithmetic operators: 
- `+`: Addition
- `-`: Subtraction
- `*`: Multiplication
- `/`: Division
- `%`: Remainder
- `^`: Exponentiation

> [!NOTE] 
> Galvan does not offer unicode alternatives for logical operators
> as `∧` and `∨` could be confused with `v` and `^` respectively.
> If you want to use unicode operators, you can define them yourself.
Logical operators:
- `and`, `&&`: Logical and
- `or`, `||`: Logical or
- `xor`, `^^`: Logical xor
- `not`, `!`: Logical not

> [!WARNING]
> Bitwise operators are implemented yet
Bitwise operators are prefixed with b:
- `b|`: Bitwise or
- `b&`: Bitwise and
- `b^`: Bitwise xor
- `b<<`: Bitwise left shift
- `b>>`: Bitwise right shift
- `b~`: Bitwise not

Comparison operators:
- `==`: Equality
- `!=`, `≠`: Inequality
- `<`: Less than
- `<=`, `≤`: Less than or equal
- `>` Greater than
- `>=`, `≥`:: Greater than or equal
- `===`, `≡`: Pointer equality, only works for heap references
- `!==`, `≢`: Pointer inequality, only works for heap references

Collection operators:
- `++`: Concatenation
- `--`: Removal
- `[]`: Indexing
- `[:]`: Slicing
- `in`, `∈`, `∊`: Membership

#### Unicode and Custom Operators
Galvan supports Unicode and custom operators:
> [!WARNING]
> Custom operators are not implemented yet
```rust
@infix("⨁")
fn xor(lhs: n, rhs: n) = lhs ^^ rhs

@prefix("√")
fn sqrt(n: Float) = n.sqrt()

main {
    let a_bool = true 
    let other_bool = false
    let value = if a_bool ⨁ other_bool { √16.0 } else { 3.0 }
}
```

This section defines custom infix `⨁` and prefix `√` operators. 
Note that no whitespace is allowed between a prefix operator and the operands.
Infix operators have to be surrounded by whitespace.

### Closures
Closures are defined using the parameter list syntax:
```rust
let add = |a, b| a + b
```

Closure types use the arrow syntax:
```rust
fn map(self: [t], f: t -> u) -> [u] {
    mut result = []
    for self {
        result.push(f(it))
    }
    result
}
```

#### Trailing Closures
Functions with trailing closures are allowed to omit the parameter list and the () around the parameter list:
```rust
iter
    .map { it * 2 }
    // Trailing closures with only one parameter can use the it keyword instead of naming it explicitly
    .filter { it % 2 == 0 }
    // The parameter list before the trailing closure can be omitted
    .reduce start |acc, e| { acc + e }
```

Trailing closures can also use numbered parameters instead of giving a parameter list
```rust
iter
    .map { #0 * 2 }
    .filter { #0 % 2 == 0 }
    .reduce start { #0 + #1 }
```

### Parentheses-Free Function Calls
In a statement or as the right-hand side of an assignment, parentheses for function calls can be omitted:
```rust
fn add(a: Int, b: Int) -> Int {
    a + b
}

main {
    let result = add 2, 3
    print result // 5
}
```
As this syntax is only allowed in assignments and statements, you cannot use it in i.e. function parameters. This avoids ambiguity.
It is also not allowed to use this syntax for functions that take no arguments to avoid confusing it with a variable.

### Testing
Every obstacle to writing unit tests is a unit test that is not written. For this reason, Galvan provides a concise syntax to quickly write unit tests in any .galvan file:
```galvan
test {
    assert 2 == 2
}

test "Ensure that addition works correctly" {
    assert 2 + 2 == 4
}
```
Like 'main', 'test' is not a function but an entry point. Tests can take a string as a description. Although this is optional, adding a brief description to your unit tests is highly encouraged.

## Semicolon Inference
> [!WARNING]
> Semicolon Inference is not yet implemented

While Galvan uses semicolons to separate statements, Galvan infers semicolons on newlines, except when:
- the next line starts with an operator (including membership operators like `.` and `?.`)
- the current line ends with an operator or a comma
- the next non-whitespace and non-newline character is a `]`, `}`, or `)`
- the last non-whitespace and non-newline character was a `[`, `{` or `(`

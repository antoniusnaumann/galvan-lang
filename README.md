# Galvan
A high-level companion language for Rust.

> [!IMPORTANT]
> This is a work in progress and under development. It currently is in a state of a hobby project and most features described here are not yet implemented.
> I am working on this project in my free time - if you like the ideas presented here, and want to help, feel free to contact me or start a discussion here on GitHub.

### A Tour of Galvan
#### Introduction to Galvan

Galvan is a modern programming language that transpiles to Rust. It provides a concise syntax while leveraging the full power of Rust's ecosystem. 

#### Basic Syntax and String Formatting
```galvan
main {
    let name = "Galvan"
    print("Welcome to {name}, the modern language!")
}
```
Note that Galvan strings always support inline format arguments.

#### Functions
Like in Rust, functions are defined with the `fn` keyword and return the value of the last expression:
```galvan
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

Very short functions can also be defined with = and have their return type inferred:
```galvan
fn add(a: Int, b: Int) = a + b
```
Those functions are not allowed to have newlines in their body.

#### Types 
Types in Galvan are defined with the `type` keyword.
```galvan
// TODO: Also use '(' here?
/// A struct definition
pub type Color {
    r: Int
    g: Int
    b: Int
}

// Structs can also use the named tuple syntax
pub type Person(name: String, age: Int)

/// A type alias
pub type Human = Person

/// A tuple type
pub type Couple(Person, Person)

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

#### Member Functions
All functions are declared top-level. If their first parameter is named `self`, they can be called as member functions:
```galvan
pub type Dog (name: String)

fn bark(self: Dog) {
    print("{self.name} barks")
}
```

#### Collections

Galvan features intuitive syntax for collections:
```galvan
pub type IntArray = [Int]
pub type StringSet = {String}
pub type MyDict = {String: Int}
pub type OrderedDict = [String: Int]
```

Ordered types use `[]`, unordered types use `{}`.

#### Optionals and Result Types
Galvan provides concise syntax for optionals and result types:

```galvan
type OptionalInt = Int?
type FileOrErr = File!
type FileOrIoErr = File!IoError
```
The error variant is specified after the `!` symbol. If it is not given, a flexible error type is used.

```galvan
fn open_file(path: String) -> File! {
    let file = File::open(path)!
    let contents = file.read_to_string()?.find("foo")?.uppercase() ?? ""
    
    contents
}
```
`!` operator unwraps the result and early returns if the result is an error. This is identical to the `?` operator in Rust.
`?` is the safe call operator in Galvan. The subsequent expression is only evaluated if the result is not an error and not none.
`??` is the null-coalescing operator, you can use it to provide a default if the left-hand side expression is none

#### Union Types
Galvan supports union types everywhere where a type identifier is expected:
```galvan
fn print_value(value: Int | String) {
    print("Value: {value}")
}
```

#### Pass-by-Value and Pass-by-Reference
By default, arguments are passed by value. If the argument needs to be mutated, the `mut` keyword can be used to pass it by reference:
For consistency, the `let` keyword is allowed as well but redundant as parameters are passed by value by default.
```galvan
fn add_one(mut value: Int) {
    value += 1
}

// Using `let` is not necessary here but allowed
fn incremented(let value : Int) -> Int {
    value + 1
} 
```

Galvan's `mut value: T` would be equivalent to Rust's `value: &mut T`. Galvan does not have immutable references, as all values are copy-on-write.
```galvan
// No copy is happening here as the value is not mutated
// Arguments are passed by value by default
fn bark_at(self: Dog, other: Dog) {
    print("{self.name} barks at {other.name}")
}
```

```galvan
// A copy is happening here as the value is mutated
fn shout_at(self: Dog, other: Dog) {
    // Redeclaring is neccessary as value parameters cannot be mutated
    let other = other
    // Copy is happening here
    other.name = other.name.uppercase()
    print("{self.name} shouts at {other.name}")
}
```

```galvan
fn grow(mut self: Dog) {
    // This mutates the original value as it is passed by reference
    self.age += 1
}
```

#### Stored References
References that are allowed to be stored in structs have to be declared as heap references, this is done by prefixing the declaration with `ref`:
```galvan
pub type Person {
    name: String
    age: Int
    // This is a heap reference
    ref dog: Dog
}

main {
    // Note that constructors use '(' with named arguments
    ref dog = Dog(name: "Bello", age: 5)
    // The `dog` field now points to the same entity as the `dog` variable 
    let person = Person(name: "Jochen, age: 67, dog: ref dog)
    dog.age += 1
    
    print(person.dog.age) // 6
    print(dog.age) // 6
}
```
Heap references use atomic reference counting to be auto-freed when no longer needed and are always mutable.
In contrast to `let` and `mut` values, `ref` values. They follow reference semantics, meaning that they point to the same object. For this reason, they are always mutable.



#### Control Flow
##### Loops
Like in Rust, loops can yield a value:
```galvan
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
```galvan
for 0..<n {
    print(it)
}
```
The loop variable is available via the `it` keyword, but can also be named explicitly using closure parameter syntax:
```galvan
for 0..<n |i| {
    print(i)
}
```

Note that ranges are declared using `..<` (exclusive upper bound) or `..=` (inclusive upper bound).

##### If-Else
```galvan
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
```galvan
try potential_error {
    print("Optional was {it}")
} else {
    print("Error occured: {it}")
}
```
The unwrapped variant is available via the it keyword, like in closures. You can also name it using closure parameter syntax to declare them explicitly:
```galvan
try potential_error |value| {
    print("Optional was {value}")
} else |error| {
    print("Error occured: {error}")
}
```

#### Return and Throw
Return values are implicit, however you can use the `return` keyword to return early:
```galvan 
fn fib(n: Int) -> Int {
    if n <= 1 {
        return n
    }
    fib(n - 1) + fib(n - 2)
}
```

Returning an error early is done using the `throw` keyword:
```galvan
fn checked_divide(a: Float, b: Float) -> Float! {
    if b == 0 {
        throw "Division by zero"
    }
    a / b
}
```

#### Generics
In Galvan, type identifiers are always starting with an upper case letter. Using a lower case letter instead introduces a type parameter:
```galvan
type Container {
    value: t
}

fn get_value(self: Container<t>) -> t {
    self.value
}
```

Bounds can be specified using the `where` keyword:
```galvan
fn concat_hash(self: t, other: t) -> t where t: Hash {
    self.hash() ++ other.hash()
}
```

#### Operators
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
- `not`, `!`: Logical not

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
- `===`: Pointer equality, only works for heap references
- `!==`: Pointer inequality, only works for heap references

Collection operators:
- `++`: Concatenation
- `--`: Removal
- `[]`: Indexing
- `[:]`: Slicing
- `in`: Membership

#### Unicode and Custom Operators
Galvan supports Unicode and custom operators:

```galvan
@infix("⨁")
fn custom_add(lhs: n, rhs: n) = lhs + rhs

@prefix("√")
fn sqrt(n: Float) = n.sqrt()

main {
    let sum = 5 ⨁ 10
    let value = √16.0
}
```

This section defines custom infix `⨁` and prefix `√` operators. 
Note that no whitespace is allowed between a prefix operator and the operands.
Infix operators have to be surrounded by whitespace.

#### Closures
Closures are defined using the parameter list syntax:
```galvan
let add = |a, b| a + b
```

Closure types use the arrow syntax:
```galvan
fn map(self: [t], f: t -> u) -> [u] {
    mut result = []
    for self {
        result.push(f(it))
    }
    result
}
```

Functions with trailing closures are allowed to omit the parameter list and the () around the parameter list:
```galvan
iter
    .map { it * 2 }
    // Trailing closures with only one parameter can use the it keyword instead of naming it explicitly
    .filter { it % 2 == 0 }
    // The parameter list before the trailing closure can be omitted
    .reduce start |acc, e| { acc + e }
```

Trailing closures can also use numbered parameters instead of giving a parameter list
```galvan
iter
    .map { #0 * 2 }
    .filter { #0 % 2 == 0 }
    .reduce start { #0 + #1 }
```

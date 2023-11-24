# Galvan
## Types
### Type Definition
In Galvan, you use the type keyword for both struct types, tuple types and type aliases.
```galvan
// No comma is needed between members if you put members on separate lines 
type MyType {
  a: TypeA
  b: TypeB
}

// Like Kotlin or Swift, Galvan has significant newlines, so you can omit the semicolon at the end of the line
type MyTuple(TypeA, TypeB)

type MyAlias = TypeA
```

### Values vs. References vs. Stored References
There are three types of parameters: value (copy-on-write), local reference (mutable) and stored reference (mutable)
- T is a value, but it will only be copied when modified inside the function, else it is just an immutable reference (&T in Rust, converted to Cow<T> when stored in a variable)
- $T is a stored reference - a mutable reference that is allowed to be stored in a struct (Arc<Mutex<T>> in Rust)
- &T is a local reference - a mutable reference that is not allowed to be stored in a struct (&mut T in Rust)

The distinction between local and stored references eliminates the need for lifetimes (but adds the overhead of reference counting to references that are contained in a struct)
This is similar to struct vs class in Swift, but in contrast to Swift, the distinction is made when declaring variables, struct members or function parameters, instead of being baked into the type definition.
If you are coming from C++, this distinction on declaration is similar to T (value types), &T (references) and *T (pointers), but with the difference that Galvan manages allocation and deallocation for you via reference counting).

```galvan
type MyType {
    a: TypeA
    b: $TypeB // stored ref
    // c: &TypeC // local ref (not allowed in structs)
}
```

### Variable Declaration
Variables are declared with the val, var and ref keywords. val and var are used for value types, ref is used for stored reference types.
```galvan
// This is an immutable value type
val a = TypeA {}
// This is a mutable value type
var b = TypeB {}
// This is a mutable stored reference type
ref c = TypeC {}

Or use:
let a = TypeA {}
mut b = TypeB {}

and for stored references:
let c = $TypeC {}
```
There is no way to declare a mutable local reference: Values are copy-on-write, so you can just use a value type instead.

## Functions
### Member functions
Types never contain functions, instead, functions are defined outside of types and can declare receiver parameters.
This is similar to Go. It also means, that (like in Kotlin and Swift) receiver functions can be defined for foreign types with the same syntax you would use to define them for your own types.

To declare a member function, use the special name self as the first parameter. Member functions can also be called as static functions with the self parameter supplied as first parameter.
This way you can also control if self should be a (mutable) reference, a copy-on-write value or a stored reference.
```galvan
// Does not need to mutate self, so it can be called on a copy-on-write value (which will not be copied in this example)
fn bark(self: Dog) {
    print("Woof!")
}

// Only available for stored references
fn befriend(self: $Dog, other: $Dog) {
    self.friends.push(other)
    other.friends.push(self)
}

// Needs a mutable reference to change
fn rename(self: &Dog, name: String) {
    self.name = name
}
```

## Examples
```galvan
type TypeA
type TypeB
type MyType {
    a: TypeA
    b: $TypeB // stored ref
}

fn print_a(a: TypeA) {
    // Like in rust, shadowing variables is allowed
    val a = a
    print(a)
}

// Short-hand syntax for one-liners with inferred return type
fn make_t(a: TypeA, b: $TypeB) => MyType(a, b)

// Usual syntax for functions with return type
fn return_something() -> TypeA {
    val a = TypeA()
    a // Implicit return at the end of the function, like in rust. To return nothing, add () at the end
}

main {
    val a = TypeA {}
    val b = TypeB {}
    val t = MyType(a, copy b) // (copy b) or (move b) possible
    // There are val (immutable value type), var (mutable value type) and ref (reference type) types
    // Reference types are always mutable, val and var types are copy-on-write, so you can pass them around and only be copied when actually modified
    
    print_a(t.a)
    
    val t_new = make_t(t.a, t.b)
}
```

Turns into this:

```rust
#[derive(Clone, Type)]
struct TypeA {}

#[derive(Clone, Type)]
struct TypeB {}

#[derive(Clone, Type)]
struct MyType {
    a: StoredVal<TypeA>,
    b: StoredRef<TypeB>,
}

fn make_t<A>(a: &A, b: StoredRef<TypeB>) -> MyType
    where
        A: AsStoredVal<Stored=TypeA> + AsLocalVal,
{
    MyType {
        a: a.as_stored_val(),
        b: b.as_stored_ref(),
    }
}

fn return_something() -> StoredVal<TypeA> {
    let a = TypeA {};
    return a;
}

fn print_a<T, A>(a: A)
    where
        A: AsStoredVal<Stored=T> + AsLocalVal,
        T: Type,
{
    let a = a.as_local_val();
    println!("{}", a);
}

fn main() {
    let a = TypeA {};
    let b = TypeB {};
    let t = MyType {
        a: a.as_stored_val(),
        b: b.as_stored_ref(),
    };

    print_a(t.a.as_local_val());

    let t_new = make_t(t.a.as_local_ref(), t.b.as_stored_ref());
}
```

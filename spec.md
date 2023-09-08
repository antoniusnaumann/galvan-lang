# Arc
## Examples
```arc
type TypeA
type TypeB
type MyType {
  a: TypeA
  b: ~TypeB
}

fn print_a(a: TypeA) {
  print(a)
}

// Short-hand syntax for one-liners with inferred return type
fn make_t(a: TypeA, b: ~TypeB) -> MyType(a, b)

// Usual syntax for functions with return type
fn return_something(): TypeA {
  val a = TypeA()
  -> a // The return operator or keyword must be used
  // as opposed to Rust there is no implicit return at the end of the function
}

main {
  val a = TypeA {}
  val b = TypeB {}
  val t = MyType(a, <:b) // <:b (copy b) or <-b (move b) possible

  print_a(t.a)

  val t_new = make_t(t.a, t.b)
}
```
or expressed with keyword syntax:
```arc
type TypeA
type TypeB
type MyType {
  a: stored val TypeA // putting the val keyword here is optional since type attributes are val types by default
  b: stored ref TypeB // the stored keyword can be ommitted here (and above) since it is implied in type attributes
}

// As for structs, you can omit the val keyword here
fn print_a(a: TypeA) {
  print(a)
}

// You can just take val instead of stored val (and leave the keyword out), since they are implicitly convertible
// You must take stored ref since a (local) ref cannot be turned into a stored ref
fn make_t(a: stored val TypeA, b: stored ref TypeB) -> MyType(a, b)

// Usual syntax for functions with return type
fn return_something(): TypeA {
  val a = TypeA()
  return a // The return operator or keyword must be used
  // as opposed to Rust there is no implicit return at the end of the function
}

main {
  val a = TypeA {}
  val b = TypeB {}
  val t = MyType(a, copy b) // You could also move b here, since it is not used afterwards. 
  // Copying (or moving) is required to make it clear that converting a val to a stored ref creates a stored ref to a newly created val and NOT to the local val

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
        b: StoredRef<TypeB>>,
    }

    fn make_t<A>(a: &A, b: StoredRef<TypeB>) -> MyType
    where
        A: AsStoredVal<Stored = TypeA> + AsLocalVal,
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

    fn print<T, A>(a: A)
    where
        A: AsStoredVal<Stored = T> + AsLocalVal,
        T: Type,
    {
        let a = a.as_local_val();
        let b = a.as_local_val();
    }

    fn main() {
        let a = TypeA {};
        let b = TypeB {};
        let t = MyType {
            a: a.as_stored_val(),
            b: b.as_stored_ref(),
        };

        print(t.a.as_local_val());

        let t_new = make_t(t.a.as_local_ref(), t.b.as_stored_ref());
    }
```
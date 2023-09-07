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

fn make_t(a: TypeA, b: ~TypeB) -> MyType(a, b)

main {
  let a = TypeA {}
  let b = TypeB {}
  let t = MyType(a, <:b) // <:b (copy b) or <-b (move b) possible

  print_a(t.a)

  let t_new = make_t(t.a, t.b)
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
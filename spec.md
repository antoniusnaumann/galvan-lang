# Arc
## Examples
```arc
struct MyType {
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
  let t = MyType(a, <: b) // <: (copy b) or <- b (move b) possible

  print_a(t.a)

  let t_new = make_t(t.a, t.b)
} 
```

Turns into this:

```rust
    use std::{
        borrow::Cow,
        sync::{Arc, Mutex},
    };
    #[derive(Clone)]
    struct TypeA {}
    #[derive(Clone)]
    struct TypeB {}

    struct MyType {
        a: TypeA,
        b: Arc<Mutex<TypeB>>,
    }

    fn make_t<A>(a: &A, b: Arc<Mutex<TypeB>>) -> MyType
    where
        A: ToOwned<Owned = TypeA>,
    {
        MyType {
            a: a.to_owned(),
            b: b.clone(),
        }
    }

    fn print<T, A>(a: &A)
    where
        A: ToOwned<Owned = T>,
    {
        let a: Cow<A> = Cow::Borrowed(a);
    }

    #[test]
    fn main() {
        let a = TypeA {};
        let b = TypeB {};
        let t = MyType {
            a: a.clone(),
            b: Arc::new(Mutex::new(b.clone())),
        };

        print(&t.a);

        let t_new = make_t(&t.a, t.b.clone());

        let x = Cow::Borrowed(&a);
        print(&x);
    }
}
```
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
struct MyType<'a> {
  a: Cow<'a, TypeA>
  b: Arc<Mutex<TypeB>>
}

fn make_t(a: Cow<'_, TypeA>, b: Arc<Mutex<TypeB>>) -> MyType {
  MyType { a: a.clone(), b: b.clone() }
}

fn main() {
  let a = Cow::from(TypeA { })
  let b = Cow::from(TypeB { })
  let t = Cow::from(MyType { 
    a: Cow::from(&a), // is there a better way to pass cow to a here? 
    b: Arc::new(Mutex::new(b.deref().clone())),
  })

  print_a(Cow::from(t.a))

  let t_new = Cow::from(make_t(t.a.clone(), t.b.clone()))
}
```

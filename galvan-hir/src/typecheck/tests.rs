use galvan_ast::{Ownership, TypeElement};
use galvan_files::Source;
use galvan_into_ast::{SegmentAst, SourceIntoAst};

use crate::hir::*;
use crate::typecheck::typecheck;

fn lower(code: &str) -> HirModule {
    let ast = Source::from_string(code)
        .try_into_ast()
        .expect("test code should parse");
    let segmented = vec![ast].segmented().expect("test code should segment");
    let (module, errors) = typecheck(segmented).expect("test code should typecheck");
    assert!(
        !errors.has_errors(),
        "expected no type errors, got: {errors}"
    );
    module
}

fn function<'m>(module: &'m HirModule, name: &str) -> &'m HirFunction {
    module
        .functions
        .iter()
        .find(|function| function.signature.identifier.as_str() == name)
        .expect("function should exist")
}

fn trailing(function: &HirFunction) -> &HirExpression {
    function
        .body
        .trailing_expression()
        .expect("function should have a trailing expression")
}

#[test]
fn infers_arithmetic_types_and_ownership() {
    let module = lower("pub fn add(a: Int, b: Int) -> Int { a + b }");
    let tail = trailing(function(&module, "add"));

    assert!(matches!(tail.kind, HirExpressionKind::Arithmetic(_)));
    let TypeElement::Plain(basic) = &tail.ty else {
        panic!("expected plain type, got {:?}", tail.ty);
    };
    assert_eq!(basic.ident.as_str(), "Int");
    // Copy result that is returned needs no adjustment
    assert!(tail.adjustments.is_empty());
    assert_eq!(tail.adjusted_ownership(), Ownership::UniqueOwned);
}

#[test]
fn copy_parameters_are_owned_and_passed_by_value() {
    let module = lower(
        "fn multiply(a: Int, b: Int) -> Int { a * b }
         fn double(a: Int) -> Int { multiply(a, 2) }",
    );
    let tail = trailing(function(&module, "double"));

    let HirExpressionKind::FunctionCall(call) = &tail.kind else {
        panic!("expected function call, got {:?}", tail.kind);
    };
    // Copy arguments are passed by value without adjustments
    assert!(call.args.iter().all(|arg| arg.adjustments.is_empty()));
}

#[test]
fn non_copy_parameters_are_borrowed() {
    let module = lower(
        "type Dog { name: String }
         fn pet(dog: Dog) {}
         fn main_fn() {
             let dog = Dog(name: \"Rex\")
             pet(dog)
         }",
    );
    let main_fn = function(&module, "main_fn");

    // The local is owned, the parameter expects a borrow -> `&dog`
    let HirStatement::Expression(call) = &main_fn.body.statements[1] else {
        panic!("expected call statement");
    };
    let HirExpressionKind::FunctionCall(call) = &call.kind else {
        panic!("expected function call");
    };
    assert_eq!(call.args[0].adjustments, vec![Adjustment::Borrow]);
}

#[test]
fn owned_locals_are_cloned_when_stored() {
    let module = lower(
        "type Dog { name: String }
         fn name_of(dog: Dog) -> String { dog.name }",
    );
    let tail = trailing(function(&module, "name_of"));

    // `dog` is a borrowed parameter, so `dog.name` is borrowed and must be
    // cloned to be returned
    assert!(matches!(tail.kind, HirExpressionKind::FieldAccess(_)));
    assert_eq!(tail.ownership, Ownership::Borrowed);
    assert_eq!(tail.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn copy_fields_need_no_clone() {
    let module = lower(
        "type Dog { age: Int }
         fn age_of(dog: Dog) -> Int { dog.age }",
    );
    let tail = trailing(function(&module, "age_of"));

    assert_eq!(tail.ownership, Ownership::UniqueOwned);
    assert!(tail.adjustments.is_empty());
}

#[test]
fn values_are_wrapped_when_optional_is_expected() {
    let module = lower("fn answer() -> Int? { 42 }");
    let tail = trailing(function(&module, "answer"));

    assert_eq!(tail.adjustments, vec![Adjustment::WrapSome]);
}

#[test]
fn if_without_else_becomes_optional() {
    let module = lower("fn answer() -> Int? { if true { 42 } }");
    let tail = trailing(function(&module, "answer"));

    let HirExpressionKind::If(if_expr) = &tail.kind else {
        panic!("expected if expression");
    };
    assert!(if_expr.wraps_optional);
    assert!(matches!(tail.ty, TypeElement::Optional(_)));
    // The branch tail is wrapped, the if itself needs no adjustment
    assert!(tail.adjustments.is_empty());
    let branch_tail = if_expr
        .then_block
        .trailing_expression()
        .expect("then block should have a trailing expression");
    assert_eq!(branch_tail.adjustments, vec![Adjustment::WrapSome]);
}

#[test]
fn for_loops_reify_into_hir_nodes() {
    let module = lower(
        "fn doubled(values: [Int]) -> [Int] {
             for values |value| { value * 2 }
         }",
    );
    let tail = trailing(function(&module, "doubled"));

    let HirExpressionKind::For(for_expr) = &tail.kind else {
        panic!("expected for expression");
    };
    // Copy elements are destructured by reference
    assert!(for_expr.bind_by_ref);
    assert!(for_expr.collect.is_some());
    // The borrowed parameter is iterated without further adjustment
    assert!(for_expr.iterable.adjustments.is_empty());
    assert!(matches!(tail.ty, TypeElement::Array(_)));
}

#[test]
fn iterating_locals_borrows_them() {
    let module = lower(
        "fn sum() -> Int {
             let values = [1, 2, 3]
             mut total = 0
             for values |value| { total += value }
             total
         }",
    );
    let body = &function(&module, "sum").body;

    let HirStatement::Expression(for_expr) = &body.statements[2] else {
        panic!("expected for statement");
    };
    let HirExpressionKind::For(for_expr) = &for_expr.kind else {
        panic!("expected for expression");
    };
    assert_eq!(for_expr.iterable.adjustments, vec![Adjustment::Borrow]);
    assert!(for_expr.collect.is_none());
}

#[test]
fn mut_parameters_are_mutably_borrowed() {
    let module = lower(
        "type Dog { age: Int }
         fn birthday(mut dog: Dog) { dog.age = dog.age + 1 }
         fn celebrate() {
             mut dog = Dog(age: 3)
             birthday(mut dog)
         }",
    );

    let birthday = function(&module, "birthday");
    let HirStatement::Assignment(assignment) = &birthday.body.statements[0] else {
        panic!("expected assignment");
    };
    // Assigning through a field of a `&mut` parameter needs no deref
    assert!(!assignment.deref_target);

    let celebrate = function(&module, "celebrate");
    let HirStatement::Expression(call) = &celebrate.body.statements[1] else {
        panic!("expected call statement");
    };
    let HirExpressionKind::FunctionCall(call) = &call.kind else {
        panic!("expected function call");
    };
    assert_eq!(call.args[0].adjustments, vec![Adjustment::MutBorrow]);
}

#[test]
fn else_unwrap_clones_borrowed_values() {
    let module = lower(
        "type Dog { name: String }
         fn good_boy(a: Dog?, b: Dog) -> Dog { a else { b } }",
    );
    let tail = trailing(function(&module, "good_boy"));

    let HirExpressionKind::ElseUnwrap(unwrap) = &tail.kind else {
        panic!("expected else unwrap");
    };
    // Borrowed receiver: match by value without a ref pattern
    assert!(!unwrap.by_ref);
    // The unwrapped borrowed value must be cloned to be returned
    assert_eq!(unwrap.value.adjustments, vec![Adjustment::ToOwned]);
    let else_tail = unwrap
        .else_block
        .trailing_expression()
        .expect("else block should have trailing expression");
    assert_eq!(else_tail.adjustments, vec![Adjustment::ToOwned]);
    // The unwrap produces an owned value, no outer adjustment needed
    assert!(tail.adjustments.is_empty());
}

#[test]
fn try_clones_shared_scrutinee() {
    let module = lower(
        "type HasOpt { a: String? }
         fn check(maybe: HasOpt) {
             try maybe.a |s| { assert s == \"Something\" } else { panic \"missing\" }
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Expression(try_expr) = &check.body.statements[0] else {
        panic!("expected try statement");
    };
    let HirExpressionKind::Try(try_expr) = &try_expr.kind else {
        panic!("expected try expression");
    };
    // Field of a borrowed parameter: match on a borrow, do not move
    assert_eq!(try_expr.condition.adjustments, vec![Adjustment::Borrow]);
    assert_eq!(try_expr.kind, TryKind::Optional);
}

#[test]
fn assert_borrows_owned_side_to_match_reference_level() {
    let module = lower(
        "type Dog { name: String }
         fn check(dog: Dog) { assert dog.name == \"Rex\" }",
    );
    let check = function(&module, "check");

    let HirStatement::Expression(assert) = &check.body.statements[0] else {
        panic!("expected assert statement");
    };
    let HirExpressionKind::Assert(assert) = &assert.kind else {
        panic!("expected assert");
    };
    let HirAssert::Eq(lhs, rhs, _) = assert.as_ref() else {
        panic!("expected assert_eq");
    };
    // lhs is a borrowed field, so the owned literal needs a borrow to match
    assert!(lhs.adjustments.is_empty());
    assert_eq!(rhs.adjustments, vec![Adjustment::Borrow]);
}

#[test]
fn constructor_arguments_are_owned() {
    let module = lower(
        "type Dog { name: String, age: Int }
         fn new_dog(name: String) -> Dog { Dog(name: name, age: 0) }",
    );
    let tail = trailing(function(&module, "new_dog"));

    let HirExpressionKind::ConstructorCall(constructor) = &tail.kind else {
        panic!("expected constructor call");
    };
    // The borrowed parameter must be cloned into the struct
    assert_eq!(constructor.args[0].1.adjustments, vec![Adjustment::ToOwned]);
    // The copy literal is moved
    assert!(constructor.args[1].1.adjustments.is_empty());
}

#[test]
fn constructor_defaults_are_materialized() {
    let module = lower(
        "type Book { title: String = \"Lorem Ipsum\" }
         fn new_book() -> Book { Book() }",
    );
    let tail = trailing(function(&module, "new_book"));

    let HirExpressionKind::ConstructorCall(constructor) = &tail.kind else {
        panic!("expected constructor call");
    };
    assert_eq!(constructor.args.len(), 1);
    assert_eq!(constructor.args[0].0.as_str(), "title");
}

#[test]
fn safe_access_style_follows_receiver_ownership() {
    let module = lower(
        "type Dog { name: String }
         fn name_of(dog: Dog?) -> String? { dog?.name }",
    );
    let tail = trailing(function(&module, "name_of"));

    let HirExpressionKind::SafeAccess(access) = &tail.kind else {
        panic!("expected safe access");
    };
    // Optional<Dog> is not copy, the parameter is borrowed -> clone out of the map
    assert_eq!(access.style, SafeAccessStyle::Clone);
    assert!(matches!(tail.ty, TypeElement::Optional(_)));
}

#[test]
fn ownership_matches_generated_rust_for_locals() {
    let module = lower(
        "type Dog { name: String }
         fn check() {
             let dog = Dog(name: \"Rex\")
             let name = dog.name
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Declaration(declaration) = &check.body.statements[1] else {
        panic!("expected declaration");
    };
    let value = declaration.value.as_ref().expect("initializer");
    // Reading a field of an owned local for storage requires a clone,
    // exactly what the generated Rust needs to compile
    assert_eq!(value.ownership, Ownership::SharedOwned);
    assert_eq!(value.adjustments, vec![Adjustment::ToOwned]);
}

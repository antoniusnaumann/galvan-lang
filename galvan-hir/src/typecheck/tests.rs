use galvan_ast::{Ownership, TypeElement};
use galvan_files::Source;
use galvan_into_ast::{SegmentAst, SourceIntoAst};

use crate::builtins::CheckBuiltins;
use crate::error::ErrorCollector;
use crate::hir::*;
use crate::typecheck::typecheck;

fn lower_with_diagnostics(code: &str) -> (HirModule, ErrorCollector) {
    let ast = Source::from_string(code)
        .try_into_ast()
        .expect("test code should parse");
    let segmented = vec![ast].segmented().expect("test code should segment");
    typecheck(segmented).expect("test code should typecheck")
}

fn lower(code: &str) -> HirModule {
    let (module, errors) = lower_with_diagnostics(code);
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
    // Copy elements from borrowed iterables are destructured by reference.
    assert_eq!(for_expr.bindings.len(), 1);
    assert!(for_expr.bindings[0].deref);
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
fn postfix_argument_modifiers_match_prefix_modifiers() {
    let module = lower(
        "type Dog { age: Int }
         fn birthday(mut dog: Dog) { dog.age = dog.age + 1 }
         fn celebrate() {
             mut prefix = Dog(age: 3)
             mut postfix = Dog(age: 4)
             birthday(mut prefix)
             birthday(postfix.mut)
         }",
    );
    let celebrate = function(&module, "celebrate");

    for statement in &celebrate.body.statements[2..] {
        let HirStatement::Expression(call) = statement else {
            panic!("expected call statement");
        };
        let HirExpressionKind::FunctionCall(call) = &call.kind else {
            panic!("expected function call");
        };
        assert_eq!(call.args[0].adjustments, vec![Adjustment::MutBorrow]);
    }
}

#[test]
fn ref_argument_modifier_supports_postfix_syntax() {
    let module = lower(
        "type Dog
         fn share(ref dog: Dog) {}
         fn check() {
             ref dog = Dog()
             share(dog.ref)
         }",
    );
    let check = function(&module, "check");
    let HirStatement::Expression(call) = &check.body.statements[1] else {
        panic!("expected call statement");
    };
    let HirExpressionKind::FunctionCall(call) = &call.kind else {
        panic!("expected function call");
    };
    assert_eq!(call.args[0].adjustments, vec![Adjustment::ArcClone]);
}

#[test]
fn ref_variables_can_be_passed_as_mutable_arguments() {
    let module = lower(
        "fn bump(mut value: Int) { value += 1 }
         fn check() {
             ref counter = 0
             bump(counter.mut)
         }",
    );
    let check = function(&module, "check");
    let HirStatement::Expression(call) = &check.body.statements[1] else {
        panic!("expected call statement");
    };
    let HirExpressionKind::FunctionCall(call) = &call.kind else {
        panic!("expected function call");
    };
    assert_eq!(
        call.args[0].adjustments,
        vec![
            Adjustment::LockRef,
            Adjustment::Deref,
            Adjustment::MutBorrow
        ]
    );
}

#[test]
fn mutable_method_receivers_accept_all_explicit_call_forms() {
    let module = lower(
        "type Dog { age: Int }
         fn birthday(mut self: Dog) { self.age = self.age + 1 }
         fn celebrate() {
             mut postfix = Dog(age: 3)
             mut grouped = Dog(age: 4)
             mut function_style = Dog(age: 5)
             postfix.mut.birthday()
             (mut grouped).birthday()
             birthday(mut function_style)
         }",
    );
    let celebrate = function(&module, "celebrate");

    for statement in &celebrate.body.statements[3..] {
        let HirStatement::Expression(call) = statement else {
            panic!("expected call statement");
        };
        let HirExpressionKind::MethodCall(call) = &call.kind else {
            panic!("expected method call");
        };
        assert_eq!(call.receiver.adjusted_ownership(), Ownership::MutBorrowed);
    }
}

#[test]
fn mutable_method_receivers_require_explicit_passing_mode() {
    let (_module, errors) = lower_with_diagnostics(
        "type Dog
         fn bark(mut self: Dog) {}
         fn check() {
             mut dog = Dog()
             dog.bark()
         }",
    );

    assert!(errors.errors().any(|diagnostic| {
        diagnostic.message
            == "Argument 'self' requires `mut` passing mode, found unmodified passing mode"
    }));
}

#[test]
fn passing_modifiers_are_rejected_for_unmodified_parameters() {
    let (_module, errors) = lower_with_diagnostics(
        "type Dog
         fn pet(dog: Dog) {}
         fn check() {
             mut dog = Dog()
             pet(dog.mut)
         }",
    );

    assert!(errors.errors().any(|diagnostic| {
        diagnostic.message
            == "Argument 'dog' requires unmodified passing mode, found `mut` passing mode"
    }));
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
    assert_eq!(
        constructor.args[0].value.adjustments,
        vec![Adjustment::ToOwned]
    );
    // The copy literal is moved
    assert!(constructor.args[1].value.adjustments.is_empty());
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
    assert_eq!(constructor.args[0].field.as_str(), "title");
}

#[test]
fn field_access_locks_ref_receiver() {
    let module = lower(
        "type Dog { name: String }
         fn name() -> String {
             ref dog = Dog(name: \"Rex\")
             dog.name
         }",
    );
    let tail = trailing(function(&module, "name"));
    let HirExpressionKind::FieldAccess(access) = &tail.kind else {
        panic!("expected field access");
    };
    assert_eq!(access.receiver.adjustments, vec![Adjustment::LockRef]);
    assert_eq!(tail.ownership, Ownership::SharedOwned);
    assert_eq!(tail.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn index_access_locks_ref_base() {
    let module = lower(
        "fn first() -> String {
             ref names = [\"Rex\"]
             names[0]
         }",
    );
    let tail = trailing(function(&module, "first"));
    let HirExpressionKind::Index(access) = &tail.kind else {
        panic!("expected index access");
    };
    assert_eq!(access.base.adjustments, vec![Adjustment::LockRef]);
    assert_eq!(tail.ownership, Ownership::SharedOwned);
    assert_eq!(tail.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn field_and_index_assignments_preserve_mutable_places() {
    let module = lower(
        "type Dog { age: Int }
         fn check() {
             mut mut_dog = Dog(age: 1)
             ref ref_dog = Dog(age: 2)
             mut mut_values = [3]
             ref ref_values = [4]
             mut_dog.age = 5
             ref_dog.age = 6
             mut_values[0] = 7
             ref_values[0] = 8
         }",
    );
    let check = function(&module, "check");
    let assignments = check.body.statements[4..]
        .iter()
        .map(|statement| match statement {
            HirStatement::Assignment(assignment) => assignment,
            _ => panic!("expected assignment"),
        })
        .collect::<Vec<_>>();

    let HirExpressionKind::FieldAccess(mut_field) = &assignments[0].target.kind else {
        panic!("expected field assignment");
    };
    assert!(mut_field.receiver.adjustments.is_empty());

    let HirExpressionKind::FieldAccess(ref_field) = &assignments[1].target.kind else {
        panic!("expected field assignment");
    };
    assert_eq!(ref_field.receiver.adjustments, vec![Adjustment::LockRef]);

    let HirExpressionKind::Index(mut_index) = &assignments[2].target.kind else {
        panic!("expected index assignment");
    };
    assert!(mut_index.base.adjustments.is_empty());

    let HirExpressionKind::Index(ref_index) = &assignments[3].target.kind else {
        panic!("expected index assignment");
    };
    assert_eq!(ref_index.base.adjustments, vec![Adjustment::LockRef]);
    assert!(assignments
        .iter()
        .all(|assignment| !assignment.deref_target));
}

#[test]
fn constructor_ref_field_modifier_shares_existing_ref() {
    let module = lower(
        "type Dog { name: String }
         type Owner { ref dog: Dog }
         fn owner() -> Owner {
             ref dog = Dog(name: \"Rex\")
             Owner(dog: ref dog)
         }",
    );
    let tail = trailing(function(&module, "owner"));

    let HirExpressionKind::ConstructorCall(constructor) = &tail.kind else {
        panic!("expected constructor call");
    };
    assert!(constructor.args[0].store_as_ref);
    assert_eq!(
        constructor.args[0].value.adjustments,
        vec![Adjustment::ArcClone]
    );
}

#[test]
fn constructor_ref_field_without_modifier_copies_into_new_ref() {
    let module = lower(
        "type Dog { name: String }
         type Owner { ref dog: Dog }
         fn owner() -> Owner {
             ref dog = Dog(name: \"Rex\")
             Owner(dog: dog)
         }",
    );
    let tail = trailing(function(&module, "owner"));

    let HirExpressionKind::ConstructorCall(constructor) = &tail.kind else {
        panic!("expected constructor call");
    };
    assert!(constructor.args[0].store_as_ref);
    assert_eq!(
        constructor.args[0].value.adjustments,
        vec![Adjustment::LockRef, Adjustment::ToOwned]
    );
}

#[test]
fn invalid_constructor_arg_modifiers_are_reported_after_parse() {
    let (_module, errors) = lower_with_diagnostics(
        "type Pair { a: Int, b: Int }
         fn pair() -> Pair { Pair(a: let 1, b: mut 2) }",
    );
    let messages = errors
        .errors()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    assert!(messages.contains(&"Invalid modifier: let is not allowed for constructor arguments"));
    assert!(messages.contains(&"Invalid modifier: mut is not allowed for constructor arguments"));
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
fn void_branches_unify_without_diagnostics() {
    let (module, errors) = lower_with_diagnostics(
        "fn check(x: Bool) {
             if x { println(\"yes\") } else { println(\"no\") }
         }",
    );
    assert!(
        errors.diagnostics().is_empty(),
        "expected no diagnostics, got: {errors}"
    );

    let check = function(&module, "check");
    let HirStatement::Expression(if_expr) = &check.body.statements[0] else {
        panic!("expected if statement");
    };
    assert!(if_expr.ty.is_void());
}

#[test]
fn statement_position_if_does_not_wrap_optional() {
    let module = lower(
        "fn check(x: Bool) {
             if x { println(\"yes\") }
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Expression(if_expr) = &check.body.statements[0] else {
        panic!("expected if statement");
    };
    let HirExpressionKind::If(inner) = &if_expr.kind else {
        panic!("expected if expression");
    };
    assert!(!inner.wraps_optional);
    assert!(if_expr.ty.is_void());
}

#[test]
fn assignment_values_are_owned_not_mut_borrowed() {
    let module = lower(
        "type Dog { name: String }
         fn rename(mut dog: Dog, name: String) { dog.name = name }",
    );
    let rename = function(&module, "rename");

    let HirStatement::Assignment(assignment) = &rename.body.statements[0] else {
        panic!("expected assignment");
    };
    // The borrowed parameter is cloned into the field; it must never be
    // adjusted to the `&mut` ownership of the target place
    assert_eq!(assignment.value.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn assignment_to_mut_parameter_dereferences_the_place() {
    let module = lower("fn overwrite(mut value: Int) { value = 42 }");
    let overwrite = function(&module, "overwrite");

    let HirStatement::Assignment(assignment) = &overwrite.body.statements[0] else {
        panic!("expected assignment");
    };
    assert!(assignment.deref_target);
    // The copy value is assigned as-is
    assert!(assignment.value.adjustments.is_empty());
}

#[test]
fn assignment_to_ref_variable_locks_the_mutex() {
    let module = lower(
        "fn check() {
             ref counter = 0
             counter = 42
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Assignment(assignment) = &check.body.statements[1] else {
        panic!("expected assignment");
    };
    // `*counter.lock().unwrap() = 42`
    assert!(assignment.deref_target);
    assert_eq!(assignment.target.adjustments, vec![Adjustment::LockRef]);
    assert!(assignment.value.adjustments.is_empty());
}

#[test]
fn assignment_ref_modifier_rebinds_ref_variable() {
    let module = lower(
        "fn check() {
             ref counter1 = 1
             ref counter2 = 0
             counter2 = ref counter1
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Assignment(assignment) = &check.body.statements[2] else {
        panic!("expected assignment");
    };
    assert!(!assignment.deref_target);
    assert!(assignment.target.adjustments.is_empty());
    assert_eq!(assignment.value.adjustments, vec![Adjustment::ArcClone]);
}

#[test]
fn invalid_assignment_rhs_modifier_is_reported_after_parse() {
    let (_module, errors) = lower_with_diagnostics(
        "fn check() {
             mut value = 1
             value = mut 2
         }",
    );

    assert!(errors.errors().any(|diagnostic| {
        diagnostic.message == "Invalid modifier: mut is not allowed for assignment right-hand sides"
    }));
}

#[test]
fn ref_declaration_modifier_shares_existing_ref() {
    let module = lower(
        "fn check() {
             ref counter1 = 1
             ref counter2 = ref counter1
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Declaration(declaration) = &check.body.statements[1] else {
        panic!("expected declaration");
    };
    let value = declaration.value.as_ref().expect("initializer");
    assert_eq!(value.adjustments, vec![Adjustment::ArcClone]);
}

#[test]
fn ref_declaration_without_assignment_modifier_copies_into_new_ref() {
    let module = lower(
        "fn check() {
             ref counter1 = 1
             ref counter2 = counter1
         }",
    );
    let check = function(&module, "check");

    let HirStatement::Declaration(declaration) = &check.body.statements[1] else {
        panic!("expected declaration");
    };
    let value = declaration.value.as_ref().expect("initializer");
    assert_eq!(
        value.adjustments,
        vec![Adjustment::LockRef, Adjustment::ToOwned]
    );
}

#[test]
fn concat_assign_classifies_and_owns_elements() {
    let module = lower(
        "fn push_name(mut names: [String], name: String) { names ++= name }
         fn merge(mut names: [String], more: [String]) { names ++= more }",
    );

    let push_name = function(&module, "push_name");
    let HirStatement::Assignment(assignment) = &push_name.body.statements[0] else {
        panic!("expected assignment");
    };
    assert_eq!(
        assignment.operator,
        HirAssignmentOperator::ConcatAssign(ConcatKind::Element)
    );
    // `push` consumes the element, so the borrowed parameter is cloned
    assert_eq!(assignment.value.adjustments, vec![Adjustment::ToOwned]);

    let merge = function(&module, "merge");
    let HirStatement::Assignment(assignment) = &merge.body.statements[0] else {
        panic!("expected assignment");
    };
    assert_eq!(
        assignment.operator,
        HirAssignmentOperator::ConcatAssign(ConcatKind::Collection)
    );
    // `extend` iterates by value, so the borrowed collection is cloned
    assert_eq!(assignment.value.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn concat_expression_owns_appended_elements() {
    let module = lower("fn appended(names: [String], name: String) -> [String] { names ++ name }");
    let tail = trailing(function(&module, "appended"));

    let HirExpressionKind::CollectionOp(operation) = &tail.kind else {
        panic!("expected collection operation");
    };
    assert_eq!(
        operation.operator,
        CollectionOperator::Concat(ConcatKind::Element)
    );
    assert_eq!(operation.rhs.adjustments, vec![Adjustment::ToOwned]);
}

#[test]
fn tuples_of_copy_types_are_copy() {
    use galvan_ast::{Span, TupleTypeItem};

    let mapping = crate::builtins::builtins();
    let int = TypeElement::Plain(galvan_ast::BasicTypeItem {
        ident: galvan_ast::TypeIdent::new("Int"),
        span: Span::default(),
    });
    let string = TypeElement::Plain(galvan_ast::BasicTypeItem {
        ident: galvan_ast::TypeIdent::new("String"),
        span: Span::default(),
    });

    let copy_tuple = TypeElement::Tuple(Box::new(TupleTypeItem {
        elements: vec![int.clone(), int.clone()],
        span: Span::default(),
    }));
    let non_copy_tuple = TypeElement::Tuple(Box::new(TupleTypeItem {
        elements: vec![int, string],
        span: Span::default(),
    }));

    assert!(mapping.is_copy(&copy_tuple));
    assert!(!mapping.is_copy(&non_copy_tuple));
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

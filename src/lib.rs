pub use galvan_transpiler::galvan_module;

#[cfg(feature = "build")]
pub use galvan_transpiler::exec::__private;

#[macro_export]
macro_rules! include {
    () => {
        include!(concat!(
            env!("OUT_DIR"),
            "/",
            ::galvan::galvan_module!("rs")
        ));
        use galvan_module::*;
    };
}

#[macro_export]
macro_rules! setup {
    () => {
        let errors = ::galvan::__private::__setup_galvan();

        if !errors.is_empty() {
            // println!("cargo:warning={}", warnings.join("\n"));
            panic!("Galvan Transpiler Error:\n{}", errors);
        }
        // TODO: How to build a rerun rule for this?
    };
}

#[macro_export]
macro_rules! main {
    () => {
        ::galvan::include!();
        fn main() {
            galvan_module::__main();
        }
    };
}

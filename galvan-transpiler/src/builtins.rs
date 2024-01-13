use crate::mapping::{mapping, Mapping};

/// Mapping of standard Galvan types to Rust types
pub fn builtins() -> Mapping {
    mapping!(
        ("Bool" => "bool", copy),

        // Hack to explicitly enable rusts inference for number literals
        ("__Number" => "_", copy),
        // Symbolic type name for partial inference
        ("__Infer" => "_"),

        ("I8" => "i8", copy),
        ("I16" => "i16", copy),
        ("I32" => "i32", copy),
        ("I64" => "i64", copy),
        ("I128" => "i128", copy),
        ("Int" => "i64", copy),
        ("ISize" => "isize", copy),

        ("U8" => "u8", copy),
        ("U16" => "u16", copy),
        ("U32" => "u32", copy),
        ("U64" => "u64", copy),
        ("U128" => "u128", copy),
        ("UInt" => "u64", copy),
        ("USize" => "usize", copy),

        ("Float" => "f32", copy),
        ("Double" => "f64", copy),

        ("String" => "String", "str"),
        ("Char" => "char", copy),
    )
}

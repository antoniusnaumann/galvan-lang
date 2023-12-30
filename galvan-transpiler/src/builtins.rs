use crate::mapping::{mapping, Mapping};

/// Mapping of standard Galvan types to Rust types
pub fn builtins() -> Mapping {
    mapping!(
        "Bool" => "bool", "&bool", "&mut bool",

        "I8" => "i8", "i8", "&mut i8",
        "I16" => "i16", "i16", "&mut i16",
        "I32" => "i32", "i32", "&mut i32",
        "I64" => "i64", "i64", "&mut i64",
        "I128" => "i128", "&i128", "&mut i128",
        "Int" => "i64", "i64", "&mut i64",
        "ISize" => "isize", "isize", "&mut isize",

        "U8" => "u8", "u8", "&mut u8",
        "U16" => "u16", "u16", "&mut u16",
        "U32" => "u32", "u32", "&mut u32",
        "U64" => "u64", "u64", "&mut u64",
        "U128" => "u128", "u128", "&mut u128",
        "UInt" => "u64", "u64", "&mut u64",
        "USize" => "usize", "usize", "&mut usize",

        "Float" => "f32", "f32", "&mut f32",
        "Double" => "f64", "f64", "&mut f64",

        "String" => "String", "&str", "&mut String",
    )
}

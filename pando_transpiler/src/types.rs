use std::collections::HashMap;

// Типы для представления строк кода
#[derive(Debug, Clone)]
pub enum ParsedLine {
    Print {
        content: String,
        comment: Option<String>,
        indent: usize,
    },
    VariableDecl {
        name: String,
        type_name: String,
        value: Option<Expression>,
        comment: Option<String>,
        indent: usize,
    },
    VariableAssign {
        name: String,
        value: Expression,
        comment: Option<String>,
        indent: usize,
    },
    Comment {
        content: String,
        indent: usize,
    },
    Empty,
}

// Тип выражения
#[derive(Debug, Clone)]
pub enum Expression {
    Literal {
        value: String,
        expr_type: String,
    },
    Variable {
        name: String,
        expr_type: String,
    },
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
        expr_type: String,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
        expr_type: String,
    },
    CompoundAssign {
        name: String,
        op: BinaryOperator,
        value: Box<Expression>,
        expr_type: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Modulo,
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
}

impl BinaryOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::FloorDivide => "//",
            BinaryOperator::Modulo => "%",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseXor => "^",
        }
    }
    
    pub fn len(&self) -> usize {
        self.as_str().len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    BitwiseNot,
}

impl Expression {
    pub fn get_type(&self) -> &str {
        match self {
            Expression::Literal { expr_type, .. } => expr_type,
            Expression::Variable { expr_type, .. } => expr_type,
            Expression::BinaryOp { expr_type, .. } => expr_type,
            Expression::UnaryOp { expr_type, .. } => expr_type,
            Expression::CompoundAssign { expr_type, .. } => expr_type,
        }
    }
}

// Маппинг типов Pando -> Rust
pub fn get_type_mapping(type_name: &str) -> Option<&'static str> {
    let mapping: HashMap<&str, &str> = [
        ("int", "i32"),
        ("int8", "i8"),
        ("int16", "i16"),
        ("int32", "i32"),
        ("int64", "i64"),
        ("int128", "i128"),
        ("int_size", "isize"),
        ("uint8", "u8"),
        ("uint16", "u16"),
        ("uint32", "u32"),
        ("uint64", "u64"),
        ("uint128", "u128"),
        ("uint_size", "usize"),
        ("float", "f32"),
        ("double", "f64"),
        ("bool", "bool"),
        ("char", "char"),
        ("str", "&str"),
        ("None", "()"),
        ("bytes", "&[u8]"),
        ("bytearray", "Vec<u8>"),
        ("string", "String"),
    ]
    .iter()
    .cloned()
    .collect();
    
    mapping.get(type_name).copied()
}

// Значения по умолчанию для типов
pub fn get_default_value(type_name: &str) -> String {
    match type_name {
        "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int_size" => "0".to_string(),
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint_size" => "0".to_string(),
        "float" => "0.0f32".to_string(),
        "double" => "0.0f64".to_string(),
        "bool" => "false".to_string(),
        "char" => "'\\0'".to_string(),
        "str" => "\"\"".to_string(),
        "None" => "()".to_string(),
        "bytes" => "b\"\"".to_string(),
        "bytearray" => "Vec::new()".to_string(),
        "string" => "String::new()".to_string(),
        _ => "0".to_string(),
    }
}

// Проверка, является ли тип числовым
pub fn is_numeric_type(type_name: &str) -> bool {
    matches!(type_name,
        "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int_size" |
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint_size" |
        "float" | "double"
    )
}

// Проверка, является ли тип целочисленным
pub fn is_integer_type(type_name: &str) -> bool {
    matches!(type_name,
        "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int_size" |
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint_size"
    )
}

// Проверка, является ли тип битовым (целочисленным без знака для битовых операций)
pub fn is_bitwise_type(type_name: &str) -> bool {
    is_integer_type(type_name)
}

// Функция для экранирования строки для Rust
pub fn escape_string_for_rust(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
    }
    result
}
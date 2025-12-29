use crate::types::{ParsedLine, Expression, BinaryOperator, UnaryOperator, get_type_mapping, get_default_value};

// Функция для генерации Rust кода из выражения
pub fn generate_expression(expr: &Expression) -> String {
    match expr {
        Expression::Literal { value, expr_type } => {
            // Для bytearray, если значение начинается с b" и тип bytearray, нужно добавить .to_vec()
            if expr_type == "bytearray" && value.starts_with("b\"") {
                format!("{}.to_vec()", value)
            } else {
                value.clone()
            }
        }
        Expression::Variable { name, .. } => name.clone(),
        Expression::BinaryOp { left, op, right, .. } => {
            let left_expr = generate_expression(left);
            let right_expr = generate_expression(right);
            let op_str = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::FloorDivide => "/", // В Rust целочисленное деление
                BinaryOperator::Modulo => "%",
                BinaryOperator::BitwiseOr => "|",
                BinaryOperator::BitwiseAnd => "&",
                BinaryOperator::BitwiseXor => "^",
            };
            format!("({} {} {})", left_expr, op_str, right_expr)
        }
        Expression::UnaryOp { op, expr, .. } => {
            let inner_expr = generate_expression(expr);
            let op_str = match op {
                UnaryOperator::Negate => "-",
                UnaryOperator::BitwiseNot => "!",
            };
            format!("({}{})", op_str, inner_expr)
        }
        Expression::CompoundAssign { name, op, value, .. } => {
            let value_expr = generate_expression(value);
            let op_str = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::FloorDivide => "/",
                BinaryOperator::Modulo => "%",
                BinaryOperator::BitwiseOr => "|",
                BinaryOperator::BitwiseAnd => "&",
                BinaryOperator::BitwiseXor => "^",
            };
            format!("{} {} {}", name, op_str, value_expr)
        }
    }
}

// Функция для генерации Rust кода из распарсенной строки
pub fn generate_rust_line(parsed: &ParsedLine) -> String {
    match parsed {
        ParsedLine::Print { content, comment, indent } => {
            let indent_str = " ".repeat(*indent);
            let mut line = format!("{}println!(\"{}\");", indent_str, content);
            if let Some(comment_text) = comment {
                if comment_text.is_empty() {
                    line.push_str(" //");
                } else {
                    line.push_str(&format!(" // {}", comment_text));
                }
            }
            line
        }
        ParsedLine::VariableDecl { name, type_name, value, comment, indent } => {
            let indent_str = " ".repeat(*indent);
            let rust_type = get_type_mapping(type_name).unwrap_or("i32");
            let rust_value = match value {
                Some(expr) => {
                    let expr_str = generate_expression(expr);
                    // Специальная обработка для bytearray с байтовыми строками
                    if type_name == "bytearray" && expr_str.starts_with("b\"") {
                        format!("{}.to_vec()", expr_str)
                    } else {
                        expr_str
                    }
                }
                None => get_default_value(type_name),
            };
            
            // Все переменные теперь объявляются с mut
            let mut line = format!("{}let mut {}: {} = {};", indent_str, name, rust_type, rust_value);
            if let Some(comment_text) = comment {
                if comment_text.is_empty() {
                    line.push_str(" //");
                } else {
                    line.push_str(&format!(" // {}", comment_text));
                }
            }
            line
        }
        ParsedLine::VariableAssign { name, value, comment, indent } => {
            let indent_str = " ".repeat(*indent);
            let value_expr = generate_expression(value);
            
            // Определяем, является ли это составным присваиванием
            let line = if let Expression::CompoundAssign { op, .. } = value {
                let op_str = match op {
                    BinaryOperator::Add => "+=",
                    BinaryOperator::Subtract => "-=",
                    BinaryOperator::Multiply => "*=",
                    BinaryOperator::Divide => "/=",
                    BinaryOperator::FloorDivide => "/=",
                    BinaryOperator::Modulo => "%=",
                    BinaryOperator::BitwiseOr => "|=",
                    BinaryOperator::BitwiseAnd => "&=",
                    BinaryOperator::BitwiseXor => "^=",
                };
                format!("{}{} {};", indent_str, name, op_str)
            } else {
                format!("{}{} = {};", indent_str, name, value_expr)
            };
            
            if let Some(comment_text) = comment {
                if comment_text.is_empty() {
                    format!("{} //", line)
                } else {
                    format!("{} // {}", line, comment_text)
                }
            } else {
                line
            }
        }
        ParsedLine::Comment { content, indent } => {
            let indent_str = " ".repeat(*indent);
            format!("{}{}", indent_str, content)
        }
        ParsedLine::Empty => "".to_string(),
    }
}
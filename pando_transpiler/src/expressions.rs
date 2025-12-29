use crate::error::TranspilerError;
use crate::types::{Expression, BinaryOperator, UnaryOperator, is_numeric_type, is_bitwise_type, is_integer_type};
use std::collections::HashMap;

// Парсинг выражения
pub fn parse_expression(
    expr: &str, 
    variables: &HashMap<String, String>,
    line_num: usize,
    column: usize
) -> Result<Expression, TranspilerError> {
    let trimmed = expr.trim();
    
    // Обработка составных операторов присваивания
    if let Some((name, op, value)) = parse_compound_assignment(trimmed) {
        if !variables.contains_key(&name) {
            return Err(TranspilerError::new(
                &format!("Переменная '{}' не объявлена", name),
                line_num,
                column,
            ));
        }
        
        let var_type = variables.get(&name).unwrap().clone();
        let value_expr = parse_expression(&value, variables, line_num, column)?;
        let value_type = value_expr.get_type().to_string();
        
        // Проверка типов
        if var_type != value_type {
            return Err(TranspilerError::new(
                &format!("Несовместимые типы: {} и {}", var_type, value_type),
                line_num,
                column,
            ));
        }
        
        return Ok(Expression::CompoundAssign {
            name,
            op,
            value: Box::new(value_expr),
            expr_type: var_type,
        });
    }
    
    // Парсинг бинарных операций
    parse_binary_expression(trimmed, variables, line_num, column)
}

// Парсинг бинарных операций
fn parse_binary_expression(
    expr: &str,
    variables: &HashMap<String, String>,
    line_num: usize,
    column: usize,
) -> Result<Expression, TranspilerError> {
    // Приоритет операций (чем выше число, тем выше приоритет)
    const PRECEDENCE: &[(BinaryOperator, &str)] = &[
        (BinaryOperator::BitwiseOr, "|"),
        (BinaryOperator::BitwiseXor, "^"),
        (BinaryOperator::BitwiseAnd, "&"),
        (BinaryOperator::Add, "+"),
        (BinaryOperator::Subtract, "-"),
        (BinaryOperator::Multiply, "*"),
        (BinaryOperator::Divide, "/"),
        (BinaryOperator::FloorDivide, "//"),
        (BinaryOperator::Modulo, "%"),
    ];
    
    // Ищем оператор с наименьшим приоритетом (с учетом скобок)
    let mut paren_count = 0;
    let mut best_pos = None;
    let mut best_op = None;
    let mut best_prec = usize::MAX;
    
    let chars: Vec<char> = expr.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '(' => paren_count += 1,
            ')' => paren_count -= 1,
            _ => {}
        }
        
        if paren_count == 0 {
            for (prec, (op, op_str)) in PRECEDENCE.iter().enumerate() {
                if check_operator_at_position(expr, i, op_str) {
                    if prec < best_prec {
                        best_pos = Some(i);
                        best_op = Some(*op);
                        best_prec = prec;
                        break;
                    }
                }
            }
        }
    }
    
    if let (Some(pos), Some(op)) = (best_pos, best_op) {
        let left = &expr[..pos];
        let right = &expr[pos + op.len()..];
        
        let left_expr = parse_binary_expression(left, variables, line_num, column)?;
        let right_expr = parse_binary_expression(right, variables, line_num, column)?;
        
        // Проверка совместимости типов
        let left_type = left_expr.get_type().to_string();
        let right_type = right_expr.get_type().to_string();
        
        if left_type != right_type {
            return Err(TranspilerError::new(
                &format!("Несовместимые типы в операции: {} и {}", left_type, right_type),
                line_num,
                column + pos,
            ));
        }
        
        // Проверка допустимости операции для типа
        if !is_operator_valid_for_type(op, &left_type) {
            return Err(TranspilerError::new(
                &format!("Операция {:?} недопустима для типа {}", op, left_type),
                line_num,
                column + pos,
            ));
        }
        
        return Ok(Expression::BinaryOp {
            left: Box::new(left_expr),
            op,
            right: Box::new(right_expr),
            expr_type: left_type,
        });
    }
    
    // Если операторов нет, парсим как унарную операцию или атомарное выражение
    parse_unary_expression(expr, variables, line_num, column)
}

// Парсинг унарных операций
fn parse_unary_expression(
    expr: &str,
    variables: &HashMap<String, String>,
    line_num: usize,
    column: usize,
) -> Result<Expression, TranspilerError> {
    let trimmed = expr.trim();
    
    // Унарный минус
    if trimmed.starts_with('-') {
        let inner = &trimmed[1..].trim();
        let inner_expr = parse_unary_expression(inner, variables, line_num, column + 1)?;
        let expr_type = inner_expr.get_type().to_string();
        
        if !is_numeric_type(&expr_type) {
            return Err(TranspilerError::new(
                &format!("Унарный минус недопустим для типа {}", expr_type),
                line_num,
                column,
            ));
        }
        
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::Negate,
            expr: Box::new(inner_expr),
            expr_type,
        });
    }
    
    // Битовая инверсия
    if trimmed.starts_with('~') {
        let inner = &trimmed[1..].trim();
        let inner_expr = parse_unary_expression(inner, variables, line_num, column + 1)?;
        let expr_type = inner_expr.get_type().to_string();
        
        if !is_bitwise_type(&expr_type) {
            return Err(TranspilerError::new(
                &format!("Битовая инверсия недопустима для типа {}", expr_type),
                line_num,
                column,
            ));
        }
        
        return Ok(Expression::UnaryOp {
            op: UnaryOperator::BitwiseNot,
            expr: Box::new(inner_expr),
            expr_type,
        });
    }
    
    // Если выражение в скобках
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len()-1].trim();
        return parse_binary_expression(inner, variables, line_num, column + 1);
    }
    
    // Атомарное выражение
    parse_atomic_expression(trimmed, variables, line_num, column)
}

// Парсинг атомарного выражения (переменная или литерал)
fn parse_atomic_expression(
    expr: &str,
    variables: &HashMap<String, String>,
    line_num: usize,
    column: usize,
) -> Result<Expression, TranspilerError> {
    // Переменная
    if variables.contains_key(expr) {
        let expr_type = variables.get(expr).unwrap().clone();
        return Ok(Expression::Variable {
            name: expr.to_string(),
            expr_type,
        });
    }
    
    // Литерал
    parse_literal(expr, line_num, column)
}

// Парсинг литерала
fn parse_literal(expr: &str, line_num: usize, column: usize) -> Result<Expression, TranspilerError> {
    let trimmed = expr.trim();
    
    // Целое число
    if let Ok(_) = trimmed.parse::<i64>() {
        return Ok(Expression::Literal {
            value: trimmed.to_string(),
            expr_type: "int".to_string(),
        });
    }
    
    // Число с плавающей точкой
    if let Ok(_) = trimmed.parse::<f64>() {
        return Ok(Expression::Literal {
            value: trimmed.to_string(),
            expr_type: "float".to_string(),
        });
    }
    
    // Булево значение
    if trimmed == "True" {
        return Ok(Expression::Literal {
            value: "true".to_string(),
            expr_type: "bool".to_string(),
        });
    }
    if trimmed == "False" {
        return Ok(Expression::Literal {
            value: "false".to_string(),
            expr_type: "bool".to_string(),
        });
    }
    
    // None
    if trimmed == "None" {
        return Ok(Expression::Literal {
            value: "()".to_string(),
            expr_type: "None".to_string(),
        });
    }
    
    // Байтовая строка (bytes)
    if trimmed.starts_with("b\"") && trimmed.ends_with('"') {
        let inner = &trimmed[2..trimmed.len()-1]; // Убираем b" и "
        let escaped = crate::types::escape_string_for_rust(inner);
        return Ok(Expression::Literal {
            value: format!("b\"{}\"", escaped),
            expr_type: "bytes".to_string(),
        });
    }
    
    // Bytearray (преобразуем в Vec<u8>)
    // В Pando для bytearray тоже можно использовать b"...", но будет преобразовано в .to_vec()
    // Но для простоты пока оставляем так же
    
    // Строка
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner = &trimmed[1..trimmed.len()-1];
        let escaped = crate::types::escape_string_for_rust(inner);
        return Ok(Expression::Literal {
            value: format!("\"{}\"", escaped),
            expr_type: "str".to_string(),
        });
    }
    
    // Символ
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 3 {
        let inner = &trimmed[1..trimmed.len()-1];
        let escaped = crate::types::escape_string_for_rust(inner);
        return Ok(Expression::Literal {
            value: format!("'{}'", escaped),
            expr_type: "char".to_string(),
        });
    }
    
    // Если это идентификатор для bytearray (например, Vec::new())
    if trimmed == "Vec::new()" || trimmed == "vec![]" {
        return Ok(Expression::Literal {
            value: "Vec::new()".to_string(),
            expr_type: "bytearray".to_string(),
        });
    }
    
    Err(TranspilerError::new(
        &format!("Некорректный литерал: {}", trimmed),
        line_num,
        column,
    ))
}

// Парсинг составного присваивания
fn parse_compound_assignment(expr: &str) -> Option<(String, BinaryOperator, String)> {
    let compound_ops = [
        ("+=", BinaryOperator::Add),
        ("-=", BinaryOperator::Subtract),
        ("*=", BinaryOperator::Multiply),
        ("/=", BinaryOperator::Divide),
        ("//=", BinaryOperator::FloorDivide),
        ("%=", BinaryOperator::Modulo),
        ("|=", BinaryOperator::BitwiseOr),
        ("&=", BinaryOperator::BitwiseAnd),
        ("^=", BinaryOperator::BitwiseXor),
    ];
    
    for (op_str, op) in &compound_ops {
        if let Some(pos) = expr.find(op_str) {
            let left = &expr[..pos].trim();
            let right = &expr[pos + op_str.len()..].trim();
            
            if !left.is_empty() && !right.is_empty() {
                return Some((left.to_string(), *op, right.to_string()));
            }
        }
    }
    
    None
}

// Проверка оператора на позиции
fn check_operator_at_position(expr: &str, pos: usize, op_str: &str) -> bool {
    if pos + op_str.len() > expr.len() {
        return false;
    }
    
    &expr[pos..pos + op_str.len()] == op_str
}

// Проверка допустимости операции для типа
fn is_operator_valid_for_type(op: BinaryOperator, type_name: &str) -> bool {
    match op {
        BinaryOperator::Add | BinaryOperator::Subtract | 
        BinaryOperator::Multiply | BinaryOperator::Divide => {
            is_numeric_type(type_name)
        }
        BinaryOperator::FloorDivide | BinaryOperator::Modulo => {
            is_integer_type(type_name)
        }
        BinaryOperator::BitwiseOr | BinaryOperator::BitwiseAnd | 
        BinaryOperator::BitwiseXor => {
            is_bitwise_type(type_name)
        }
    }
}
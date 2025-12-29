use crate::error::TranspilerError;
use crate::types::{ParsedLine, get_type_mapping};
use crate::expressions::parse_expression;
use std::collections::HashMap;

// Функция для разделения строки на код и комментарий
pub fn split_code_and_comment(line: &str) -> (String, Option<String>) {
    let mut in_string = false;
    let mut escaped = false;
    let mut code_part = String::new();
    let mut comment_start = None;
    
    for c in line.chars() {
        if comment_start.is_some() {
            break;
        }
        
        if escaped {
            code_part.push(c);
            escaped = false;
            continue;
        }
        
        match c {
            '\\' => {
                escaped = true;
                code_part.push(c);
            }
            '"' | '\'' => {
                in_string = !in_string;
                code_part.push(c);
            }
            '#' => {
                if !in_string {
                    comment_start = Some(code_part.len());
                } else {
                    code_part.push(c);
                }
            }
            _ => {
                code_part.push(c);
            }
        }
    }
    
    let comment_part = if comment_start.is_some() {
        let comment_chars: String = line.chars()
            .skip(code_part.chars().count() + 1)
            .collect();
        Some(comment_chars)
    } else {
        None
    };
    
    (code_part, comment_part)
}

// Функция для парсинга одной строки
pub fn parse_line(
    line: &str, 
    line_num: usize, 
    variables: &mut HashMap<String, String>
) -> Result<ParsedLine, TranspilerError> {
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    let (code_part, comment_part) = split_code_and_comment(line);
    
    let trimmed_code = code_part.trim();
    let comment_trimmed = comment_part.map(|c| c.trim_start().to_string());
    
    // Обработка пустых строк
    if trimmed_code.is_empty() {
        if let Some(comment) = &comment_trimmed {
            if comment.is_empty() {
                return Ok(ParsedLine::Comment {
                    content: "//".to_string(),
                    indent,
                });
            } else {
                return Ok(ParsedLine::Comment {
                    content: format!("// {}", comment),
                    indent,
                });
            }
        } else {
            return Ok(ParsedLine::Empty);
        }
    }
    
    // Проверяем, начинается ли строка с print
    if trimmed_code.starts_with("print") {
        // Проверяем наличие скобок
        if !trimmed_code.contains('(') || !trimmed_code.contains(')') {
            return Err(TranspilerError::new(
                "Отсутствуют скобки у вызова print",
                line_num,
                trimmed_code.find('p').unwrap_or(1),
            ));
        }

        // Извлекаем аргументы из скобок
        let args_start = trimmed_code.find('(').unwrap();
        let args_end = trimmed_code.find(')').unwrap();
        let args = &trimmed_code[args_start + 1..args_end].trim();

        // Проверяем что аргумент - строка в двойных кавычках
        if !args.starts_with('"') || !args.ends_with('"') {
            return Err(TranspilerError::new(
                "Аргумент print должен быть строкой в двойных кавычках",
                line_num,
                args_start + 1,
            ));
        }

        // Извлекаем содержимое строки (без кавычек)
        let string_content = &args[1..args.len() - 1];
        let escaped_content = crate::types::escape_string_for_rust(string_content);
        
        return Ok(ParsedLine::Print {
            content: escaped_content,
            comment: comment_trimmed,
            indent,
        });
    }
    
    // Пытаемся распарсить как объявление переменной
    // Формат: имя: тип [= значение]
    if let Some(colon_pos) = trimmed_code.find(':') {
        let var_name = trimmed_code[..colon_pos].trim().to_string();
        
        // Проверяем корректность имени переменной
        if var_name.is_empty() {
            return Err(TranspilerError::new(
                "Отсутствует имя переменной",
                line_num,
                1,
            ));
        }
        
        if !var_name.chars().next().unwrap().is_alphabetic() {
            return Err(TranspilerError::new(
                "Имя переменной должно начинаться с буквы",
                line_num,
                1,
            ));
        }
        
        let after_colon = trimmed_code[colon_pos + 1..].trim();
        
        // Ищем тип и опциональное значение
        let parts: Vec<&str> = after_colon.splitn(2, '=').collect();
        let type_part = parts[0].trim();
        
        // Проверяем, что тип известен
        if get_type_mapping(type_part).is_none() {
            return Err(TranspilerError::new(
                &format!("Неизвестный тип: {}", type_part),
                line_num,
                colon_pos + 2,
            ));
        }
        
        // Добавляем переменную в таблицу символов
        variables.insert(var_name.clone(), type_part.to_string());
        
        let value = if parts.len() > 1 {
            let value_str = parts[1].trim();
            Some(parse_expression(value_str, variables, line_num, colon_pos + parts[0].len() + 2)?)
        } else {
            None
        };
        
        return Ok(ParsedLine::VariableDecl {
            name: var_name,
            type_name: type_part.to_string(),
            value,
            comment: comment_trimmed,
            indent,
        });
    }

    // Пытаемся распарсить как присваивание: x = значение или составное присваивание
    if let Some(equals_pos) = trimmed_code.find('=') {
        let left_side = trimmed_code[..equals_pos].trim();
        let right_side = trimmed_code[equals_pos + 1..].trim();
        
        // Проверяем, что слева от = допустимое имя переменной
        if !left_side.is_empty() && left_side.chars().next().unwrap().is_alphabetic() {
            // Проверяем, объявлена ли переменная
            if !variables.contains_key(left_side) {
                return Err(TranspilerError::new(
                    &format!("Переменная '{}' не объявлена", left_side),
                    line_num,
                    1,
                ));
            }
            
            // Получаем тип переменной
            let var_type = variables.get(left_side).unwrap().clone();
            
            // Парсим выражение
            let value = parse_expression(right_side, variables, line_num, equals_pos + 1)?;
            let value_type = value.get_type().to_string();
            
            // Проверяем совместимость типов
            if var_type != value_type {
                return Err(TranspilerError::new(
                    &format!("Несовместимые типы: нельзя присвоить {} в {}", value_type, var_type),
                    line_num,
                    equals_pos + 1,
                ));
            }
            
            return Ok(ParsedLine::VariableAssign {
                name: left_side.to_string(),
                value,
                comment: comment_trimmed,
                indent,
            });
        }
    }
    
    Err(TranspilerError::new(
        "Нераспознанная конструкция. Ожидается print, объявление или присваивание переменной",
        line_num,
        1,
    ))
}
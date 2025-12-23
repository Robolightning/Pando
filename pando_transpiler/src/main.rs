use std::fs;
use std::collections::HashMap;

// Вспомогательная структура для ошибок трансляции
#[derive(Debug)]
struct TranspilerError {
    message: String,
    line: usize,
    column: usize,
}

impl TranspilerError {
    fn new(message: &str, line: usize, column: usize) -> Self {
        Self {
            message: message.to_string(),
            line,
            column,
        }
    }
}

// Типы для представления строк кода
enum ParsedLine {
    Print {
        content: String,
        comment: Option<String>,
        indent: usize,
    },
    VariableDecl {
        name: String,
        type_name: String,
        value: Option<String>,
        comment: Option<String>,
        indent: usize,
    },
    Comment {
        content: String,
        indent: usize,
    },
    Empty,
}

// Маппинг типов Pando -> Rust
fn get_type_mapping(type_name: &str) -> Option<&'static str> {
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
    ]
    .iter()
    .cloned()
    .collect();
    
    mapping.get(type_name).copied()
}

// Значения по умолчанию для типов
fn get_default_value(type_name: &str) -> String {
    match type_name {
        "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int_size" => "0".to_string(),
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint_size" => "0".to_string(),
        "float" => "0.0f32".to_string(),
        "double" => "0.0f64".to_string(),
        "bool" => "false".to_string(),
        "char" => "'\\0'".to_string(),
        "str" => "\"\"".to_string(),
        "None" => "()".to_string(),
        _ => "0".to_string(),
    }
}

// Функция для экранирования строки для Rust
fn escape_string_for_rust(s: &str) -> String {
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

// Функция для разделения строки на код и комментарий
fn split_code_and_comment(line: &str) -> (String, Option<String>) {
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

// Парсинг значения в зависимости от типа
fn parse_value(value_str: &str, type_name: &str) -> Result<String, TranspilerError> {
    let trimmed = value_str.trim();
    
    match type_name {
        "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int_size" |
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint_size" => {
            // Проверяем, что это число
            if trimmed.parse::<i64>().is_ok() {
                Ok(trimmed.to_string())
            } else {
                Err(TranspilerError::new(
                    &format!("Некорректное числовое значение для типа {}", type_name),
                    1, 1
                ))
            }
        }
        "float" | "double" => {
            // Проверяем, что это число с плавающей точкой
            if trimmed.parse::<f64>().is_ok() {
                if type_name == "float" {
                    Ok(format!("{}f32", trimmed))
                } else {
                    Ok(format!("{}f64", trimmed))
                }
            } else {
                Err(TranspilerError::new(
                    &format!("Некорректное значение с плавающей точкой для типа {}", type_name),
                    1, 1
                ))
            }
        }
        "bool" => {
            match trimmed {
                "True" => Ok("true".to_string()),
                "False" => Ok("false".to_string()),
                _ => Err(TranspilerError::new(
                    "Булево значение должно быть True или False",
                    1, 1
                ))
            }
        }
        "char" => {
            if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 3 {
                let inner = &trimmed[1..trimmed.len()-1];
                let escaped = escape_string_for_rust(inner);
                Ok(format!("'{}'", escaped))
            } else {
                Err(TranspilerError::new(
                    "Значение char должно быть в одинарных кавычках",
                    1, 1
                ))
            }
        }
        "str" => {
            if trimmed.starts_with('"') && trimmed.ends_with('"') {
                let inner = &trimmed[1..trimmed.len()-1];
                let escaped = escape_string_for_rust(inner);
                Ok(format!("\"{}\"", escaped))
            } else {
                Err(TranspilerError::new(
                    "Строковое значение должно быть в двойных кавычках",
                    1, 1
                ))
            }
        }
        "None" => {
            match trimmed {
                "()" => Ok("()".to_string()),
                "None" => Ok("()".to_string()),  // Добавляем поддержку None
                _ => Err(TranspilerError::new(
                    "Для типа None допустимо только значение None или ()",
                    1, 1
                ))
            }
        }
        _ => Err(TranspilerError::new(
            &format!("Неизвестный тип: {}", type_name),
            1, 1
        ))
    }
}

// Функция для парсинга одной строки
fn parse_line(line: &str, line_num: usize) -> Result<ParsedLine, TranspilerError> {
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
        let escaped_content = escape_string_for_rust(string_content);
        
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
        
        let value = if parts.len() > 1 {
            let value_str = parts[1].trim();
            Some(parse_value(value_str, type_part)?)
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
    
    Err(TranspilerError::new(
        "Нераспознанная конструкция. Ожидается print или объявление переменной",
        line_num,
        1,
    ))
}

// Функция для генерации Rust кода из распарсенной строки
fn generate_rust_line(parsed: &ParsedLine) -> String {
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
            let rust_value = value.as_ref().map_or_else(
                || get_default_value(type_name),
                |v| v.clone()
            );
            
            let mut line = format!("{}let {}: {} = {};", indent_str, name, rust_type, rust_value);
            if let Some(comment_text) = comment {
                if comment_text.is_empty() {
                    line.push_str(" //");
                } else {
                    line.push_str(&format!(" // {}", comment_text));
                }
            }
            line
        }
        ParsedLine::Comment { content, indent } => {
            let indent_str = " ".repeat(*indent);
            format!("{}{}", indent_str, content)
        }
        ParsedLine::Empty => "".to_string(),
    }
}

// Основная функция трансляции
fn transpile_pd_to_rs(input_path: &str, output_path: &str) -> Result<(), TranspilerError> {
    let content = fs::read_to_string(input_path)
        .map_err(|e| TranspilerError::new(&format!("Ошибка чтения файла: {}", e), 1, 1))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut rust_lines = Vec::new();
    
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;
        
        match parse_line(line, line_num) {
            Ok(parsed) => rust_lines.push(parsed),
            Err(e) => return Err(e),
        }
    }
    
    // Проверяем, что есть хотя бы одна команда для выполнения
    let has_executable_code = rust_lines.iter().any(|line| {
        matches!(line, ParsedLine::Print { .. } | ParsedLine::VariableDecl { .. })
    });
    
    if !has_executable_code {
        return Err(TranspilerError::new(
            "Файл не содержит команд для выполнения",
            1,
            1,
        ));
    }

    // Генерация Rust кода
    let mut rust_code = String::from("fn main() {\n");
    
    for parsed in rust_lines {
        let line = generate_rust_line(&parsed);
        if line.is_empty() {
            rust_code.push('\n');
        } else {
            rust_code.push_str(&format!("    {}\n", line));
        }
    }
    
    rust_code.push('}');

    fs::write(output_path, rust_code)
        .map_err(|e| TranspilerError::new(&format!("Ошибка записи файла: {}", e), 1, 1))?;

    println!("✅ Трансляция успешно завершена!");
    println!("📁 Результат сохранён в: {}", output_path);

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 3 {
        eprintln!("❌ Использование: {} <input.pd> <output.rs>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];

    println!("🎯 Начинаю трансляцию {} -> {}", input_file, output_file);

    match transpile_pd_to_rs(input_file, output_file) {
        Ok(_) => {
            println!("\n✅ Трансляция успешна. Файл: {}", output_file);
        }
        Err(e) => {
            eprintln!(
                "❌ Ошибка трансляции в строке {}:{}: {}",
                e.line, e.column, e.message
            );
            std::process::exit(1);
        }
    }
}
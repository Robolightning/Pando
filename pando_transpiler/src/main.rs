mod types;
mod error;
mod parser;
mod generator;
mod expressions;

use std::fs;
use std::collections::HashMap;
use crate::error::TranspilerError;
use crate::parser::parse_line;
use crate::generator::generate_rust_line;

// Основная функция трансляции
fn transpile_pd_to_rs(input_path: &str, output_path: &str) -> Result<(), TranspilerError> {
    let content = fs::read_to_string(input_path)
        .map_err(|e| TranspilerError::new(&format!("Ошибка чтения файла: {}", e), 1, 1))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut rust_lines = Vec::new();
    let mut variables = HashMap::new();
    
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;
        
        match parse_line(line, line_num, &mut variables) {
            Ok(parsed) => rust_lines.push(parsed),
            Err(e) => return Err(e),
        }
    }
    
    // Проверяем, что есть хотя бы одна команда для выполнения
    let has_executable_code = rust_lines.iter().any(|line| {
        matches!(line, types::ParsedLine::Print { .. } | types::ParsedLine::VariableDecl { .. })
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
                "❌ Ошибка трансляции: {}",
                e
            );
            std::process::exit(1);
        }
    }
}
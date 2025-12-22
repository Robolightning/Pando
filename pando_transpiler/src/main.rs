use std::fs;

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

// Основная функция трансляции
fn transpile_pd_to_rs(input_path: &str, output_path: &str) -> Result<(), TranspilerError> {
    // Чтение исходного файла
    let content = fs::read_to_string(input_path)
        .map_err(|e| TranspilerError::new(&format!("Ошибка чтения файла: {}", e), 1, 1))?;

    // Разбиваем на строки для анализа
    let lines: Vec<&str> = content.lines().collect();

    // Проверка 1: отступы на верхнем уровне (строка 1, символ 1 если есть пробел)
    if !lines.is_empty() && lines[0].starts_with(char::is_whitespace) {
        return Err(TranspilerError::new(
            "На верхнем уровне не должно быть отступов",
            1,
            lines[0].find(|c: char| !c.is_whitespace()).unwrap_or(1),
        ));
    }

    // Проверка 2-5: анализ единственной строки с print
    if lines.len() != 1 {
        return Err(TranspilerError::new(
            "На данном этапе файл должен содержать ровно одну строку",
            1,
            1,
        ));
    }

    let line = lines[0].trim(); // Убираем пробелы по краям для анализа

    // Проверяем что строка начинается с print
    if !line.starts_with("print") {
        return Err(TranspilerError::new(
            "Ожидается вызов функции print",
            1,
            1,
        ));
    }

    // Проверяем наличие скобок
    if !line.contains('(') || !line.contains(')') {
        return Err(TranspilerError::new(
            "Отсутствуют скобки у вызова print",
            1,
            line.find('p').unwrap_or(1),
        ));
    }

    // Извлекаем аргументы из скобок
    let args_start = line.find('(').unwrap();
    let args_end = line.find(')').unwrap();
    let args = &line[args_start + 1..args_end].trim();

    // Проверяем что аргумент - строка в двойных кавычках
    if !args.starts_with('"') || !args.ends_with('"') {
        return Err(TranspilerError::new(
            "Аргумент print должен быть строкой в двойных кавычках",
            1,
            args_start + 1,
        ));
    }

    // Извлекаем содержимое строки (без кавычек)
    let string_content = &args[1..args.len() - 1];

    // Генерация Rust кода
    let rust_code = format!(
        "fn main() {{\n    println!(\"{}\");\n}}",
        string_content
    );

    // Запись в файл
    fs::write(output_path, rust_code)
        .map_err(|e| TranspilerError::new(&format!("Ошибка записи файла: {}", e), 1, 1))?;

    println!("✅ Трансляция успешно завершена!");
    println!("📁 Результат сохранён в: {}", output_path);

    Ok(())
}

fn main() {
    // Получаем аргументы командной строки
    let args: Vec<String> = std::env::args().collect();
    
    // Проверяем, что передано два аргумента: входной и выходной файл
    if args.len() != 3 {
        eprintln!("❌ Использование: {} <input.pd> <output.rs>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];

    println!("🎯 Начинаю трансляцию {} -> {}", input_file, output_file);

    // Шаг 1: Трансляция
    match transpile_pd_to_rs(input_file, output_file) {
        Ok(_) => {
            println!("\n✅ Трансляция успешна. Файл: {}", output_file);
            // Только трансляция, компиляцией занимается расширение VSCode
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
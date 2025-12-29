use std::fmt;

// Вспомогательная структура для ошибок трансляции
#[derive(Debug)]
pub struct TranspilerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl TranspilerError {
    pub fn new(message: &str, line: usize, column: usize) -> Self {
        Self {
            message: message.to_string(),
            line,
            column,
        }
    }
}

impl fmt::Display for TranspilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Строка {}:{} - {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for TranspilerError {}
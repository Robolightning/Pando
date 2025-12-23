import {
    createConnection,
    TextDocuments,
    Diagnostic,
    DiagnosticSeverity,
    ProposedFeatures,
    InitializeParams,
    CompletionItem,
    CompletionItemKind,
    TextDocumentPositionParams,
    TextDocumentSyncKind,
    InitializeResult,
    DidChangeConfigurationNotification,
    DidChangeTextDocumentParams,
    PublishDiagnosticsParams
} from 'vscode-languageserver/node';
import { TextDocument } from 'vscode-languageserver-textdocument';

const connection = createConnection(ProposedFeatures.all);
const documents: TextDocuments<TextDocument> = new TextDocuments(TextDocument);

// Список допустимых типов Pando
const pandoTypes = [
    'int', 'int8', 'int16', 'int32', 'int64', 'int128', 'int_size',
    'uint8', 'uint16', 'uint32', 'uint64', 'uint128', 'uint_size',
    'float', 'double', 'bool', 'char', 'str', 'None'
];

// Карта соответствия типов Pando -> Rust
const typeMap: { [key: string]: string } = {
    'int': 'i32',
    'int8': 'i8',
    'int16': 'i16',
    'int32': 'i32',
    'int64': 'i64',
    'int128': 'i128',
    'int_size': 'isize',
    'uint8': 'u8',
    'uint16': 'u16',
    'uint32': 'u32',
    'uint64': 'u64',
    'uint128': 'u128',
    'uint_size': 'usize',
    'float': 'f32',
    'double': 'f64',
    'bool': 'bool',
    'char': 'char',
    'str': '&str',
    'None': '()'
};

connection.onInitialize((params: InitializeParams) => {
    const result: InitializeResult = {
        capabilities: {
            textDocumentSync: TextDocumentSyncKind.Incremental,
            completionProvider: {
                resolveProvider: false,
                triggerCharacters: ['.', ':', '=', '(', '"', "'"]
            },
            diagnosticProvider: {
                interFileDependencies: false,
                workspaceDiagnostics: false
            }
        }
    };
    return result;
});

// Функция для валидации документа и поиска неизвестных типов
function validateTextDocument(textDocument: TextDocument): Diagnostic[] {
    const text = textDocument.getText();
    const lines = text.split('\n');
    const diagnostics: Diagnostic[] = [];

    // Регулярное выражение для поиска объявлений переменных с типами
    // Формат: имя: тип [= значение]
    const typeDeclarationRegex = /([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*([a-zA-Z_][a-zA-Z0-9_]*)/g;

    lines.forEach((line, lineIndex) => {
        // Пропускаем комментарии
        if (line.trim().startsWith('#')) {
            return;
        }

        // Ищем все объявления типов в строке
        let match: RegExpExecArray | null;
        while ((match = typeDeclarationRegex.exec(line)) !== null) {
            const varName = match[1];
            const typeName = match[2];
            const startIndex = match.index + match[0].indexOf(typeName);
            
            // Проверяем, известен ли тип
            if (!pandoTypes.includes(typeName)) {
                const diagnostic: Diagnostic = {
                    severity: DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: startIndex },
                        end: { line: lineIndex, character: startIndex + typeName.length }
                    },
                    message: `Неизвестный тип "${typeName}"`,
                    source: 'pando',
                    code: 'undefined-type'
                };
                diagnostics.push(diagnostic);
            }
        }

        // Дополнительная проверка: если строка содержит двоеточие, но нет известного типа
        const colonIndex = line.indexOf(':');
        if (colonIndex !== -1) {
            const afterColon = line.substring(colonIndex + 1).trim();
            
            // Если после двоеточия есть текст, но это не известный тип
            if (afterColon.length > 0) {
                const firstWord = afterColon.split(/\s+/)[0];
                if (!pandoTypes.includes(firstWord) && !firstWord.includes('=') && !firstWord.includes('#')) {
                    // Проверяем, что это действительно похоже на тип, а не на что-то другое
                    if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(firstWord)) {
                        const startIndex = colonIndex + 1 + line.substring(colonIndex + 1).indexOf(firstWord);
                        const diagnostic: Diagnostic = {
                            severity: DiagnosticSeverity.Error,
                            range: {
                                start: { line: lineIndex, character: startIndex },
                                end: { line: lineIndex, character: startIndex + firstWord.length }
                            },
                            message: `Неизвестный тип "${firstWord}". Допустимые типы: ${pandoTypes.join(', ')}`,
                            source: 'pando',
                            code: 'undefined-type'
                        };
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    });

    return diagnostics;
}

// Обработчик изменения текста документа
documents.onDidChangeContent((change) => {
    // change.document - это TextDocument, который изменился
    const diagnostics = validateTextDocument(change.document);
    
    // Отправляем диагностику в VS Code
    connection.sendDiagnostics({
        uri: change.document.uri,
        diagnostics
    });
});

// Обработчик открытия документа
documents.onDidOpen((open) => {
    // open.document - это TextDocument, который открыли
    const diagnostics = validateTextDocument(open.document);
    connection.sendDiagnostics({
        uri: open.document.uri,
        diagnostics
    });
});

// Автодополнение
connection.onCompletion((textDocumentPosition: TextDocumentPositionParams): CompletionItem[] => {
    const document = documents.get(textDocumentPosition.textDocument.uri);
    if (!document) {
        return [];
    }

    const position = textDocumentPosition.position;
    const lineText = document.getText({
        start: { line: position.line, character: 0 },
        end: { line: position.line, character: position.character }
    });

    const trimmed = lineText.trim();
    
    // Пропускаем строки с комментариями
    if (trimmed.startsWith('#')) {
        return [];
    }

    const completions: CompletionItem[] = [];

    // Автодополнение для print
    if (trimmed.length === 0 || 
        trimmed.endsWith('p') || 
        trimmed.endsWith('pr') || 
        trimmed.endsWith('pri') || 
        trimmed.endsWith('prin')) {
        completions.push({
            label: 'print',
            kind: CompletionItemKind.Function,
            detail: 'Pando built-in function',
            documentation: 'Выводит текст в консоль\n\nprint("текст")'
        });
    }

    // Автодополнение для типов (если есть двоеточие)
    if (trimmed.includes(':') && !trimmed.includes('=')) {
        const afterColon = trimmed.split(':').pop()?.trim() || '';
        
        // Если после двоеточия мало символов, предлагаем типы
        if (afterColon.length < 3) {
            pandoTypes.forEach(typeName => {
                if (typeName.startsWith(afterColon)) {
                    completions.push({
                        label: typeName,
                        kind: CompletionItemKind.TypeParameter,
                        detail: `Pando тип → ${typeMap[typeName] || typeName}`,
                        documentation: `Соответствует Rust типу: ${typeMap[typeName] || typeName}`
                    });
                }
            });
        }
    }

    // Автодополнение для значений bool
    if (trimmed.endsWith('=') || trimmed.includes('bool') && trimmed.includes('=')) {
        completions.push({
            label: 'True',
            kind: CompletionItemKind.Value,
            detail: 'Логическое значение',
            documentation: 'Соответствует Rust: true'
        });
        completions.push({
            label: 'False',
            kind: CompletionItemKind.Value,
            detail: 'Логическое значение',
            documentation: 'Соответствует Rust: false'
        });
    }

    // Автодополнение для значений None
    if (trimmed.includes('None') && trimmed.includes('=')) {
        completions.push({
            label: 'None',
            kind: CompletionItemKind.Value,
            detail: 'Значение для типа None',
            documentation: 'Соответствует Rust: ()'
        });
        completions.push({
            label: '()',
            kind: CompletionItemKind.Value,
            detail: 'Альтернативное значение для None',
            documentation: 'Явное указание пустого кортежа'
        });
    }

    return completions;
});

// Вспомогательная функция для получения Rust-эквивалента типа
function getRustType(pandoType: string): string {
    return typeMap[pandoType] || pandoType;
}

documents.listen(connection);
connection.listen();
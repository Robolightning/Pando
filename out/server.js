"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const node_1 = require("vscode-languageserver/node");
const vscode_languageserver_textdocument_1 = require("vscode-languageserver-textdocument");
const connection = (0, node_1.createConnection)(node_1.ProposedFeatures.all);
const documents = new node_1.TextDocuments(vscode_languageserver_textdocument_1.TextDocument);
// Список допустимых типов Pando
const pandoTypes = [
    'int', 'int8', 'int16', 'int32', 'int64', 'int128', 'int_size',
    'uint8', 'uint16', 'uint32', 'uint64', 'uint128', 'uint_size',
    'float', 'double', 'bool', 'char', 'str', 'bytes', 'bytearray', 'string', 'None'
];
// Карта соответствия типов Pando -> Rust
const typeMap = {
    'int': 'i32', 'int8': 'i8', 'int16': 'i16', 'int32': 'i32', 'int64': 'i64',
    'int128': 'i128', 'int_size': 'isize', 'uint8': 'u8', 'uint16': 'u16',
    'uint32': 'u32', 'uint64': 'u64', 'uint128': 'u128', 'uint_size': 'usize',
    'float': 'f32', 'double': 'f64', 'bool': 'bool', 'char': 'char',
    'str': '&str', 'bytes': '&[u8]', 'bytearray': 'Vec<u8>', 'string': 'String',
    'None': '()'
};
const documentVariables = new Map();
const documentVarMap = new Map();
const documentTokens = new Map(); // Новое хранилище для токенов
connection.onInitialize((params) => {
    const result = {
        capabilities: {
            textDocumentSync: node_1.TextDocumentSyncKind.Incremental,
            completionProvider: {
                resolveProvider: false,
                triggerCharacters: ['.', ':', '=', '(', '"', "'"]
            },
            semanticTokensProvider: {
                full: true,
                range: false,
                legend: {
                    tokenTypes: ['variable', 'type', 'function', 'keyword'],
                    tokenModifiers: ['declaration']
                }
            }
        }
    };
    return result;
});
// Функция для определения типа литерала
function getLiteralType(literal) {
    // Целочисленные литералы
    if (/^-?\d+$/.test(literal))
        return 'int';
    // Числа с плавающей точкой
    if (/^-?\d+\.\d+$/.test(literal) || /^-?\d+\.$/.test(literal) || /^-?\.\d+$/.test(literal)) {
        return 'float';
    }
    // Строки в двойных кавычках
    if ((literal.startsWith('"') && literal.endsWith('"')) ||
        (literal.startsWith("'") && literal.endsWith("'"))) {
        return 'str';
    }
    // Булевы значения
    if (literal === 'True' || literal === 'False')
        return 'bool';
    // None
    if (literal === 'None')
        return 'None';
    // Символы (одиночный символ в кавычках)
    if ((literal.startsWith("'") && literal.endsWith("'") && literal.length === 3) ||
        (literal.startsWith('"') && literal.endsWith('"') && literal.length === 3)) {
        return 'char';
    }
    return null;
}
// Функция для проверки совместимости типов
function areTypesCompatible(targetType, sourceType) {
    // Если типы одинаковые, всегда совместимы
    if (targetType === sourceType)
        return true;
    // Числовые преобразования (можно добавлять больше правил)
    const numericConversions = {
        'int': ['int8', 'int16', 'int32', 'int64', 'int128', 'int_size', 'float', 'double'],
        'int8': ['int16', 'int32', 'int64', 'int128', 'int_size', 'float', 'double'],
        'int16': ['int32', 'int64', 'int128', 'int_size', 'float', 'double'],
        'int32': ['int64', 'int128', 'float', 'double'],
        'int64': ['int128', 'float', 'double'],
        'float': ['double'],
    };
    return numericConversions[sourceType]?.includes(targetType) || false;
}
// Функция для парсинга документа
function parseDocument(textDocument) {
    const text = textDocument.getText();
    const lines = text.split('\n');
    const variables = [];
    const tokens = [];
    const diagnostics = [];
    const localVarMap = new Map();
    // Сначала собираем все объявления (без проверки значений)
    lines.forEach((line, lineIndex) => {
        if (line.trim().startsWith('#'))
            return;
        // Обрабатываем объявления с типами: x: тип = значение
        const declMatch = line.match(/^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*([a-zA-Z_][a-zA-Z0-9_]*)(?:\s*=\s*(.+))?/);
        if (declMatch) {
            const varName = declMatch[1];
            const typeName = declMatch[2];
            const startIndex = line.indexOf(varName);
            // Добавляем токен для объявления переменной
            tokens.push({
                line: lineIndex,
                startChar: startIndex,
                length: varName.length,
                tokenType: 0, // variable
                tokenModifiers: 1 // declaration
            });
            // Добавляем токен для типа
            const typeStart = line.indexOf(typeName);
            if (typeStart !== -1) {
                tokens.push({
                    line: lineIndex,
                    startChar: typeStart,
                    length: typeName.length,
                    tokenType: 1, // type
                    tokenModifiers: 0
                });
            }
            // Проверяем тип
            if (!pandoTypes.includes(typeName)) {
                const typeStart = line.indexOf(typeName);
                diagnostics.push({
                    severity: node_1.DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: typeStart },
                        end: { line: lineIndex, character: typeStart + typeName.length }
                    },
                    message: `Неизвестный тип "${typeName}"`,
                    source: 'pando',
                    code: 'undefined-type'
                });
            }
            else {
                const varInfo = {
                    name: varName,
                    type: typeName,
                    line: lineIndex,
                    startChar: startIndex,
                    endChar: startIndex + varName.length
                };
                if (localVarMap.has(varName)) {
                    const existing = localVarMap.get(varName);
                    diagnostics.push({
                        severity: node_1.DiagnosticSeverity.Warning,
                        range: {
                            start: { line: lineIndex, character: startIndex },
                            end: { line: lineIndex, character: startIndex + varName.length }
                        },
                        message: `Переменная "${varName}" уже объявлена ранее (строка ${existing.line + 1})`,
                        source: 'pando',
                        code: 'duplicate-variable'
                    });
                }
                else {
                    variables.push(varInfo);
                    localVarMap.set(varName, varInfo);
                }
            }
        }
    });
    // Затем проверяем присваивания и находим использования переменных
    lines.forEach((line, lineIndex) => {
        if (line.trim().startsWith('#'))
            return;
        // Находим все идентификаторы в строке
        const identifierRegex = /([a-zA-Z_][a-zA-Z0-9_]*)/g;
        let match;
        while ((match = identifierRegex.exec(line)) !== null) {
            const identifier = match[1];
            const startChar = match.index;
            // Пропускаем ключевые слова и типы
            if (pandoTypes.includes(identifier) ||
                identifier === 'print' ||
                identifier === 'True' ||
                identifier === 'False' ||
                identifier === 'None') {
                continue;
            }
            // Проверяем, является ли это использованием переменной
            if (localVarMap.has(identifier)) {
                // Проверяем, не является ли это объявлением
                const isDeclaration = variables.some(v => v.line === lineIndex &&
                    v.startChar === startChar);
                if (!isDeclaration) {
                    // Добавляем токен для использования переменной
                    tokens.push({
                        line: lineIndex,
                        startChar,
                        length: identifier.length,
                        tokenType: 0, // variable
                        tokenModifiers: 0 // не объявление
                    });
                }
            }
        }
        // Проверяем присваивания в объявлениях: x: тип = значение
        const declWithAssignMatch = line.match(/^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(.+)/);
        if (declWithAssignMatch) {
            const varName = declWithAssignMatch[1];
            const typeName = declWithAssignMatch[2];
            const value = declWithAssignMatch[3].trim();
            // Пропускаем, если тип неизвестен (уже есть ошибка выше)
            if (!pandoTypes.includes(typeName))
                return;
            // Проверяем, является ли значение переменной
            const varMatch = value.match(/^([a-zA-Z_][a-zA-Z0-9_]*)$/);
            if (varMatch) {
                const rhsVarName = varMatch[1];
                const rhsVar = localVarMap.get(rhsVarName);
                if (!rhsVar) {
                    const rhsStart = line.indexOf(rhsVarName);
                    diagnostics.push({
                        severity: node_1.DiagnosticSeverity.Error,
                        range: {
                            start: { line: lineIndex, character: rhsStart },
                            end: { line: lineIndex, character: rhsStart + rhsVarName.length }
                        },
                        message: `Переменная "${rhsVarName}" не объявлена`,
                        source: 'pando',
                        code: 'undeclared-variable'
                    });
                }
                else if (!areTypesCompatible(typeName, rhsVar.type)) {
                    const rhsStart = line.indexOf(rhsVarName);
                    diagnostics.push({
                        severity: node_1.DiagnosticSeverity.Error,
                        range: {
                            start: { line: lineIndex, character: rhsStart },
                            end: { line: lineIndex, character: rhsStart + rhsVarName.length }
                        },
                        message: `Несовместимые типы: нельзя присвоить ${rhsVar.type} в ${typeName}`,
                        source: 'pando',
                        code: 'type-mismatch'
                    });
                }
            }
            else {
                // Это литерал - проверяем его тип
                const literalType = getLiteralType(value);
                if (literalType !== null && !areTypesCompatible(typeName, literalType)) {
                    const valueStart = line.indexOf(value);
                    diagnostics.push({
                        severity: node_1.DiagnosticSeverity.Error,
                        range: {
                            start: { line: lineIndex, character: valueStart },
                            end: { line: lineIndex, character: valueStart + value.length }
                        },
                        message: `Несовместимые типы: нельзя присвоить ${literalType} в ${typeName}`,
                        source: 'pando',
                        code: 'type-mismatch'
                    });
                }
            }
        }
        // Проверяем простые присваивания: x = y
        const simpleAssignMatch = line.match(/^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([a-zA-Z_][a-zA-Z0-9_]*)/);
        if (simpleAssignMatch && !line.includes(':')) {
            const varName = simpleAssignMatch[1];
            const rhsVarName = simpleAssignMatch[2];
            const lhsVar = localVarMap.get(varName);
            const rhsVar = localVarMap.get(rhsVarName);
            if (!lhsVar) {
                const startIndex = line.indexOf(varName);
                diagnostics.push({
                    severity: node_1.DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: startIndex },
                        end: { line: lineIndex, character: startIndex + varName.length }
                    },
                    message: `Переменная "${varName}" не объявлена`,
                    source: 'pando',
                    code: 'undeclared-variable'
                });
            }
            if (!rhsVar) {
                const startIndex = line.indexOf(rhsVarName);
                diagnostics.push({
                    severity: node_1.DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: startIndex },
                        end: { line: lineIndex, character: startIndex + rhsVarName.length }
                    },
                    message: `Переменная "${rhsVarName}" не объявлена`,
                    source: 'pando',
                    code: 'undeclared-variable'
                });
            }
            if (lhsVar && rhsVar && !areTypesCompatible(lhsVar.type, rhsVar.type)) {
                const startIndex = line.indexOf(rhsVarName);
                diagnostics.push({
                    severity: node_1.DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: startIndex },
                        end: { line: lineIndex, character: startIndex + rhsVarName.length }
                    },
                    message: `Несовместимые типы: нельзя присвоить ${rhsVar.type} в ${lhsVar.type}`,
                    source: 'pando',
                    code: 'type-mismatch'
                });
            }
        }
        // Проверяем использования переменных в print
        const printMatch = line.match(/print\s*\(\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\)/);
        if (printMatch) {
            const varName = printMatch[1];
            if (!localVarMap.has(varName)) {
                const startIndex = line.indexOf(varName);
                diagnostics.push({
                    severity: node_1.DiagnosticSeverity.Error,
                    range: {
                        start: { line: lineIndex, character: startIndex },
                        end: { line: lineIndex, character: startIndex + varName.length }
                    },
                    message: `Переменная "${varName}" не объявлена`,
                    source: 'pando',
                    code: 'undeclared-variable'
                });
            }
        }
    });
    return { variables, tokens, diagnostics };
}
// Обработчик изменения документа
documents.onDidChangeContent((change) => {
    const { variables, tokens, diagnostics } = parseDocument(change.document);
    documentVariables.set(change.document.uri, variables);
    documentTokens.set(change.document.uri, tokens); // Сохраняем токены
    const varMap = new Map();
    variables.forEach(v => varMap.set(v.name, v));
    documentVarMap.set(change.document.uri, varMap);
    // Отправляем диагностику
    connection.sendDiagnostics({
        uri: change.document.uri,
        diagnostics
    });
    // Запрашиваем обновление семантических токенов
    connection.sendNotification('workspace/semanticTokens/refresh');
});
// Семантические токены
connection.onRequest('textDocument/semanticTokens/full', (params) => {
    const builder = new node_1.SemanticTokensBuilder();
    const tokens = documentTokens.get(params.textDocument.uri) || [];
    // Сортируем токены по строкам и позициям
    tokens.sort((a, b) => {
        if (a.line === b.line) {
            return a.startChar - b.startChar;
        }
        return a.line - b.line;
    });
    // Добавляем токены в билдер
    tokens.forEach(token => {
        builder.push(token.line, token.startChar, token.length, token.tokenType, token.tokenModifiers);
    });
    return builder.build();
});
// Автодополнение
connection.onCompletion((textDocumentPosition) => {
    const document = documents.get(textDocumentPosition.textDocument.uri);
    if (!document)
        return [];
    const position = textDocumentPosition.position;
    const lineText = document.getText({
        start: { line: position.line, character: 0 },
        end: { line: position.line, character: position.character }
    });
    const trimmed = lineText.trim();
    if (trimmed.startsWith('#'))
        return [];
    const completions = [];
    const varMap = documentVarMap.get(textDocumentPosition.textDocument.uri);
    // Автодополнение для print
    if (trimmed.length === 0 || trimmed.match(/\b(print|p|pr|pri|prin)$/)) {
        completions.push({
            label: 'print',
            kind: node_1.CompletionItemKind.Function,
            detail: 'Pando built-in function',
            documentation: 'Выводит текст в консоль\n\nprint("текст")'
        });
    }
    // Автодополнение для типов
    if (trimmed.includes(':') && !trimmed.includes('=')) {
        const afterColon = trimmed.split(':').pop()?.trim() || '';
        if (afterColon.length < 3) {
            pandoTypes.forEach(typeName => {
                if (typeName.startsWith(afterColon)) {
                    completions.push({
                        label: typeName,
                        kind: node_1.CompletionItemKind.TypeParameter,
                        detail: `Pando тип → ${typeMap[typeName]}`,
                        documentation: `Соответствует Rust типу: ${typeMap[typeName]}`
                    });
                }
            });
        }
    }
    // Автодополнение для переменных
    if (varMap && (trimmed.endsWith('=') || trimmed.endsWith(' ') || trimmed.length === 0)) {
        Array.from(varMap.keys()).forEach(varName => {
            const varInfo = varMap.get(varName);
            completions.push({
                label: varName,
                kind: node_1.CompletionItemKind.Variable,
                detail: `Тип: ${varInfo.type}`,
                documentation: `Объявлена в строке ${varInfo.line + 1}`
            });
        });
    }
    // Автодополнение для bool значений
    if (trimmed.endsWith('=') || (trimmed.includes('bool') && trimmed.includes('='))) {
        completions.push({ label: 'True', kind: node_1.CompletionItemKind.Value, detail: 'Логическое значение' });
        completions.push({ label: 'False', kind: node_1.CompletionItemKind.Value, detail: 'Логическое значение' });
    }
    // Автодополнение для None значений
    if (trimmed.includes('None') && trimmed.includes('=')) {
        completions.push({ label: 'None', kind: node_1.CompletionItemKind.Value, detail: 'Значение для типа None' });
        completions.push({ label: '()', kind: node_1.CompletionItemKind.Value, detail: 'Альтернативное значение' });
    }
    return completions;
});
documents.listen(connection);
connection.listen();
//# sourceMappingURL=server.js.map
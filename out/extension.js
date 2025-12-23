"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = require("vscode");
const path = require("path");
const fs = require("fs");
const child_process_1 = require("child_process");
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    console.log('Расширение Pando активировано!');
    // === 1. ЗАПУСК LSP-СЕРВЕРА ===
    // Путь к скомпилированному серверу
    const serverModule = context.asAbsolutePath(path.join('out', 'server.js'));
    // Опции для отладки (будем использовать в разработке)
    const debugOptions = { execArgv: ['--nolazy', '--inspect=6009'] };
    // Опции сервера
    const serverOptions = {
        run: {
            module: serverModule,
            transport: node_1.TransportKind.ipc
        },
        debug: {
            module: serverModule,
            transport: node_1.TransportKind.ipc,
            options: debugOptions
        }
    };
    // Опции клиента
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'pd' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.pd')
        }
    };
    // Создаём и запускаем LSP-клиент
    client = new node_1.LanguageClient('pandoLanguageServer', 'Pando Language Server', serverOptions, clientOptions);
    // Запускаем клиент
    client.start();
    // === 2. СТАТУСНАЯ ПАНЕЛЬ ===
    // Создаем кнопку на панели статуса
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.command = 'pando.run';
    statusBarItem.text = '$(play) Run Pando';
    statusBarItem.tooltip = 'Запустить текущий Pando файл';
    // Показываем кнопку только для .pd файлов
    if (vscode.window.activeTextEditor?.document.languageId === 'pd') {
        statusBarItem.show();
    }
    // Следим за сменой активного редактора
    vscode.window.onDidChangeActiveTextEditor(editor => {
        if (editor?.document.languageId === 'pd') {
            statusBarItem.show();
        }
        else {
            statusBarItem.hide();
        }
    });
    // === 3. РЕГИСТРАЦИЯ КОМАНДЫ "pando.run" (ваш существующий код) ===
    const runCommand = vscode.commands.registerCommand('pando.run', async (fileUri) => {
        try {
            // 1. Определяем файл для компиляции
            const targetFile = fileUri || vscode.window.activeTextEditor?.document.uri;
            if (!targetFile) {
                vscode.window.showErrorMessage('Нет активного файла для компиляции');
                return;
            }
            if (path.extname(targetFile.fsPath) !== '.pd') {
                vscode.window.showErrorMessage('Можно компилировать только .pd файлы');
                return;
            }
            // 2. Показываем статус
            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Компиляция Pando...',
                cancellable: false
            }, async (progress) => {
                progress.report({ message: 'Трансляция в Rust...' });
                // 3. Получаем путь к вашему транслятору Rust
                const extensionPath = context.extensionPath;
                const transpilerPath = path.join(extensionPath, 'pando_transpiler', 'target', 'release', 'pando_transpiler.exe');
                // 4. Вызываем транслятор
                const pdFile = targetFile.fsPath;
                const rsFile = pdFile.replace('.pd', '.rs');
                const outputChannel = vscode.window.createOutputChannel('Pando Compiler');
                outputChannel.show();
                outputChannel.appendLine(`🚀 Компиляция ${pdFile}`);
                // Запускаем процесс трансляции
                await runTranspiler(transpilerPath, pdFile, rsFile, outputChannel);
                progress.report({ message: 'Компиляция Rust...' });
                // 5. Компилируем Rust код
                const rustcResult = await compileRust(rsFile, outputChannel);
                if (!rustcResult.success) {
                    throw new Error('Ошибка компиляции Rust');
                }
                progress.report({ message: 'Запуск программы...' });
                // 6. Запускаем скомпилированную программу
                const exePath = rsFile.replace('.rs', '');
                await runExecutable(exePath, outputChannel);
                vscode.window.showInformationMessage('✅ Программа успешно выполнена!');
            });
        }
        catch (error) {
            vscode.window.showErrorMessage(`❌ Ошибка компиляции: ${error.message}`);
        }
    });
    // === 4. ДОБАВЛЯЕМ ВСЕ ПОДПИСКИ В КОНТЕКСТ ===
    context.subscriptions.push(runCommand, statusBarItem);
}
// Функция для запуска транслятора
async function runTranspiler(transpilerPath, inputFile, outputFile, outputChannel) {
    return new Promise((resolve, reject) => {
        // Проверяем существование транслятора
        if (!fs.existsSync(transpilerPath)) {
            outputChannel.appendLine(`⚠️ Транслятор не найден по пути: ${transpilerPath}`);
            outputChannel.appendLine('Собираю транслятор...');
            // Пытаемся собрать транслятор
            const cargoPath = path.join(path.dirname(transpilerPath), '..', '..'); // путь к корню проекта
            const cargo = (0, child_process_1.spawn)('cargo', ['build', '--release'], { cwd: cargoPath });
            cargo.stdout.on('data', (data) => outputChannel.append(data.toString()));
            cargo.stderr.on('data', (data) => outputChannel.append(data.toString()));
            cargo.on('close', (code) => {
                if (code === 0 && fs.existsSync(transpilerPath)) {
                    runTranspilerProcess();
                }
                else {
                    reject(new Error('Не удалось собрать транслятор'));
                }
            });
        }
        else {
            runTranspilerProcess();
        }
        function runTranspilerProcess() {
            const process = (0, child_process_1.spawn)(transpilerPath, [inputFile, outputFile]);
            process.stdout.on('data', (data) => outputChannel.append(data.toString()));
            process.stderr.on('data', (data) => outputChannel.append(data.toString()));
            process.on('close', (code) => {
                if (code === 0) {
                    outputChannel.appendLine(`✅ Трансляция завершена: ${outputFile}`);
                    resolve();
                }
                else {
                    reject(new Error(`Транслятор завершился с кодом ${code}`));
                }
            });
        }
    });
}
// Функция для компиляции Rust кода
async function compileRust(rsFile, outputChannel) {
    return new Promise((resolve) => {
        const rsDir = path.dirname(rsFile); // Получаем директорию файла
        const exePath = rsFile.replace('.rs', ''); // Путь к будущему исполняемому файлу (без расширения)
        // Для Windows добавляем расширение .exe
        const exePathWithExt = (process.platform === 'win32') ? exePath + '.exe' : exePath;
        outputChannel.appendLine(`🔧 Компиляция Rust: rustc ${rsFile} (в директории ${rsDir})`);
        // Указываем рабочую директорию и полный путь для выходного файла
        const rustc = (0, child_process_1.spawn)('rustc', [rsFile, '-o', exePathWithExt], { cwd: rsDir });
        rustc.stdout.on('data', (data) => outputChannel.append(data.toString()));
        rustc.stderr.on('data', (data) => outputChannel.append(data.toString()));
        rustc.on('close', (code) => {
            if (code === 0) {
                outputChannel.appendLine('✅ Rust компиляция успешна');
                resolve({ success: true });
            }
            else {
                outputChannel.appendLine('❌ Ошибка компиляции Rust');
                resolve({ success: false });
            }
        });
    });
}
// Функция для запуска исполняемого файла
async function runExecutable(exePath, outputChannel) {
    return new Promise((resolve, reject) => {
        // Для Windows добавляем расширение .exe
        const exePathWithExt = (process.platform === 'win32') ? exePath + '.exe' : exePath;
        outputChannel.appendLine(`🚀 Запуск программы: ${exePathWithExt}`);
        outputChannel.appendLine('='.repeat(50));
        const childProcess = (0, child_process_1.spawn)(exePathWithExt, [], { shell: true });
        childProcess.stdout.on('data', (data) => outputChannel.append(data.toString()));
        childProcess.stderr.on('data', (data) => outputChannel.append(data.toString()));
        childProcess.on('close', (code) => {
            outputChannel.appendLine('='.repeat(50));
            if (code === 0) {
                outputChannel.appendLine('✅ Программа выполнена успешно');
                resolve();
            }
            else {
                reject(new Error(`Программа завершилась с кодом ${code}`));
            }
        });
    });
}
function deactivate() {
    // Останавливаем LSP-клиент при деактивации
    if (client) {
        return client.stop();
    }
}
//# sourceMappingURL=extension.js.map
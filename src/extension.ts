import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext) {
    const config = workspace.getConfiguration('basica.lsp');
    const enabled = config.get<boolean>('enabled', true);

    if (!enabled) {
        console.log('basica LSP is disabled');
        return;
    }

    // Get server path from configuration or use bundled binary
    let serverPath = config.get<string>('path');

    if (!serverPath) {
        // Use bundled server based on platform
        const ext = process.platform === 'win32' ? '.exe' : '';
        serverPath = context.asAbsolutePath(
            path.join('server', 'basica-lsp' + ext)
        );
    }

    const serverExecutable: Executable = {
        command: serverPath,
        args: [],
    };

    const serverOptions: ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'basica' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.bas'),
        },
    };

    client = new LanguageClient(
        'basicaLanguageServer',
        'basica Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

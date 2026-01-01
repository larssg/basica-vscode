# basica-vscode

VS Code extension for [basica](https://github.com/larssg/basica) - an IBM BASIC interpreter.

## Features

- **Syntax highlighting** for `.bas` files
- **Diagnostics** - Parse errors shown as you type
- **Go to Definition** - Ctrl+click on GOTO/GOSUB line numbers to jump to target
- **Hover documentation** - Hover over keywords and functions for help

## Installation

### From VSIX

1. Download the `.vsix` file from releases
2. In VS Code: Extensions → ... → Install from VSIX

### From Source

```bash
# Clone the repo
git clone https://github.com/larssg/basica-vscode
cd basica-vscode

# Build the LSP server
cd server
cargo build --release
cd ..

# Install dependencies and build extension
npm install
npm run compile

# Package
npx vsce package
```

## Development

```bash
# Watch mode for TypeScript
npm run watch

# Open in VS Code and press F5 to launch Extension Development Host
code .
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `basica.lsp.enabled` | `true` | Enable/disable the language server |
| `basica.lsp.path` | `""` | Custom path to basica-lsp binary |

## License

MIT

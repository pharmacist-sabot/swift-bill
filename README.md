# Swift Bill

[![Tauri](https://img.shields.io/badge/Tauri-2.0-24c8db?logo=tauri&logoColor=fff)](https://tauri.app/)
[![Vue.js](https://img.shields.io/badge/Vue-3.0-4FC08D?logo=vue.js&logoColor=fff)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-f74c00?logo=rust&logoColor=fff)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-3178C6?logo=typescript&logoColor=fff)](https://www.typescriptlang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

A specialized desktop application designed for hospitals to automate the generation of pharmaceutical disbursement reports. Swift Bill replaces error-prone manual Excel workflows with a robust, reliable, and fast desktop application natively connected to legacy database systems.

## Features

- **Direct Database Integration**: Connects directly to legacy MS SQL Server databases (INVS) using native TDS without the need for ODBC drivers.
- **Native PDF Generation**: Generates print-ready precise A4 landscape and portrait PDF reports directly in the application using `printpdf`.
- **Embedded Thai Typography**: Ships with embedded Thai fonts (CordiaNew) ensuring consistent rendering on any workstation.
- **Multi-Round Processing**: Supports incremental batch processing (รอบ) tracking running balances, request numbers, and document continuity safely.
- **Modern Stack**: Built for speed and reliability with Rust at the core (Tauri shell + business logic) and a responsive fluid UI built in Vue 3 (Composition API).

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Bun](https://bun.sh/) (v1.3+)
- Tauri CLI dependencies for your OS (see [Tauri Setup Guide](https://tauri.app/v1/guides/getting-started/prerequisites))

### Starting the Development Server

1. Clone the repository:

   ```bash
   git clone https://github.com/suradet-ps/swift-bill.git
   cd swift-bill
   ```

2. Install frontend dependencies:

   ```bash
   bun install
   ```

3. Run the development server (starts Vue and the Tauri desktop window):

   ```bash
   bun run tauri dev
   ```

## Production Build

To build the application for release:

```bash
bun run tauri build
```

This will produce native executables in `src-tauri/target/release/bundle`.

## Contributing

We welcome contributions! Please see our [Contributing Guide](docs/CONTRIBUTING.md) for details on how to get started.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
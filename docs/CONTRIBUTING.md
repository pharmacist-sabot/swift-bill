# Contributing to Swift Bill

First off, thank you for considering contributing to Swift Bill! We appreciate your time and effort in helping improve this project.

## Development Workflow

This application uses **Tauri**, combining a **Rust** backend with a **Vue 3 (TypeScript)** frontend. All business logic, database queries, and PDF generation occur in Rust, while the user interface and state are managed with Vue.

### 1. Identify an Issue

Check the issue tracker to see if the feature or bug you want to work on has already been reported. If not, open a new issue describing what you'd like to do.

### 2. Set Up the Environment

Make sure you have Bun and Rust installed as described in the `../README.md`.

### 3. Branching

Create a branch for your feature or bug fix:

```bash
git checkout -b feature/your-feature-name
```

or

```bash
git checkout -b fix/your-bug-fix
```

### 4. Code Guidelines

- **Rust**:
  - Keep logic in Rust. Vue should only be a thin UI layer.
  - Follow the existing error-handling patterns (`Result<T, String>` for cross-boundary errors).
  - Use `cargo fmt` before committing.
  - Avoid modifying the `INVS` database (the app is strictly read-only mode for legacy systems).
- **Frontend (Vue/TS)**:
  - Use standard Composition API (`<script setup>`).
  - Keep types strictly mirrored with Rust definitions.
  - Use simple, self-contained components. Let the UI be focused and professional.

### 5. Committing

Please write clear, concise commit messages. Prefix your commits with their domain, e.g.,

- `feat: update PDF logic for receiving summaries`
- `fix: correct remaining budget calculation carry-over`
- `ui: improve spacing in the settings modal`

### 6. Submitting a Pull Request

Once you are happy with your changes, push your branch and open a Pull Request (PR) against the `main` branch.
Provide a clear description of the problem solved and any UI changes made if applicable.

## Found a Bug?

If you find a bug in the source code or a mistake in the documentation, you can help us by submitting an issue to our GitHub Repository. Even better, you can submit a Pull Request with a fix.

Thank you for your contributions!

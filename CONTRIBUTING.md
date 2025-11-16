# Contributing to Claude Code Mux

Thank you for your interest in contributing to Claude Code Mux! We appreciate all contributions, from bug reports to new features.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Coding Guidelines](#coding-guidelines)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Be kind, professional, and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/claude-code-mux
   cd claude-code-mux
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/9j/claude-code-mux
   ```
4. **Create a branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)

### Build and Run

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run in development mode
cargo run

# Run with release optimizations
cargo run --release

# Format code
cargo fmt

# Check for linting issues
cargo clippy
```

### Project Structure

```
claude-code-mux/
├── src/
│   ├── main.rs              # Application entry point
│   ├── cli/                 # CLI argument parsing
│   ├── server/              # HTTP server and admin UI
│   ├── router/              # Routing logic
│   ├── providers/           # Provider implementations
│   └── models/              # Data models
├── config/                  # Configuration templates
├── docs/                    # Documentation
└── benches/                 # Benchmarks
```

## How to Contribute

### Reporting Bugs

Before creating a bug report:
1. Check the [existing issues](https://github.com/9j/claude-code-mux/issues)
2. Try the latest version from `main` branch

When creating a bug report, include:
- **Title**: Clear, descriptive summary
- **Description**: Detailed explanation of the issue
- **Steps to reproduce**: Numbered list of exact steps
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Environment**:
  - OS and version
  - Rust version (`rustc --version`)
  - Claude Code Mux version
- **Logs**: Relevant error messages or stack traces
- **Screenshots**: If applicable

### Suggesting Features

Feature suggestions are welcome! Please:
1. Check [discussions](https://github.com/9j/claude-code-mux/discussions) first
2. Clearly describe the use case
3. Explain why this feature would be useful
4. Consider backward compatibility
5. Be open to discussion and feedback

### Pull Requests

We actively welcome pull requests for:
- Bug fixes
- New features
- Documentation improvements
- Performance optimizations
- Code refactoring
- Test coverage improvements

## Coding Guidelines

### Rust Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/)
- Use `cargo fmt` before committing
- Ensure `cargo clippy` passes with no warnings
- Write idiomatic Rust code
- Add documentation comments for public APIs

### Code Quality

```rust
// Good: Clear, documented public API
/// Routes a request to the appropriate model based on routing rules.
///
/// # Arguments
/// * `request` - The incoming API request
///
/// # Returns
/// The model name to use for this request
pub fn route_request(request: &Request) -> String {
    // Implementation
}

// Good: Descriptive variable names
let selected_provider = providers.iter()
    .find(|p| p.enabled && p.priority == 1)
    .unwrap_or(&default_provider);

// Good: Error handling
match config.load() {
    Ok(cfg) => cfg,
    Err(e) => {
        error!("Failed to load config: {}", e);
        return Err(AppError::ConfigError(e));
    }
}
```

### Admin UI Guidelines

When modifying the Admin UI (`src/server/admin.html`):

1. **Follow design principles**: See [docs/design-principles.md](docs/design-principles.md)
2. **Use URL-based routing**: See [docs/url-state-management.md](docs/url-state-management.md)
3. **Update localStorage properly**: See [docs/localstorage-state-management.md](docs/localstorage-state-management.md)
4. **Key rules**:
   - One purpose per page
   - Show value before complexity
   - Make questions easy to answer
   - Always show save notifications

### Documentation

- Document all public APIs with `///` comments
- Include examples in documentation
- Update README.md for user-facing changes
- Add inline comments for complex logic
- Keep docs up to date with code changes

## Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```
feat(router): add support for custom routing rules

Allow users to define custom routing rules in the config file.
This enables more flexible routing strategies.

Closes #123
```

```
fix(server): resolve admin UI state sync issue

The admin UI was not properly syncing localStorage to server
on save. This commit ensures syncToServer() is called correctly.

Fixes #456
```

```
docs: update installation instructions for macOS

Add Homebrew installation method and clarify build steps.
```

## Pull Request Process

1. **Update your fork**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Make your changes**:
   - Write clear, focused commits
   - Add tests for new features
   - Update documentation

3. **Test thoroughly**:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create Pull Request**:
   - Use a clear, descriptive title
   - Reference related issues
   - Describe what changed and why
   - Add screenshots for UI changes
   - Check "Allow edits from maintainers"

6. **Address review feedback**:
   - Be responsive to comments
   - Make requested changes
   - Push updates to the same branch
   - Re-request review when ready

7. **Merge requirements**:
   - All tests passing
   - Code review approved
   - No merge conflicts
   - Follows style guidelines

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_logic() {
        let router = Router::new();
        let request = create_test_request();

        let result = router.route(request);

        assert_eq!(result.model, "expected-model");
    }
}
```

### Test Coverage

- Aim for >80% coverage on new code
- Test edge cases and error conditions
- Include integration tests for major features

## Questions?

- **General questions**: [GitHub Discussions](https://github.com/9j/claude-code-mux/discussions)
- **Bug reports**: [GitHub Issues](https://github.com/9j/claude-code-mux/issues)
- **Security issues**: See [SECURITY.md](SECURITY.md) (create if sensitive)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Claude Code Mux! 🎉

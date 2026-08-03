# KeefeOS Contributing Guidelines

Thank you for your interest in contributing to KeefeOS! This document provides guidelines and instructions for contributing.

## Code of Conduct

### Our Pledge

We pledge to make participation in KeefeOS a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Expected Behavior

- Be respectful and inclusive
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment or discrimination
- Trolling or insulting comments
- Publishing others' private information
- Other unethical or unprofessional conduct

### Reporting

Report violations to: conduct@keefeos.org

---

## How to Contribute

### 1. Fork and Clone

```bash
git clone https://github.com/your-username/keefeos.git
cd keefeos
git remote add upstream https://github.com/keefeos/keefeos.git
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 3. Make Changes

Follow our coding standards (see below).

### 4. Test Your Changes

```bash
# Build the kernel
cargo build -p keefeos-kernel --target x86_64-unknown-none

# Run tests
cargo test --workspace

# Check for common issues
cargo clippy -- -D warnings
cargo fmt --check
cargo audit
```

### 5. Commit Your Changes

```bash
git commit -m "feat: add your feature description

Detailed description of what was changed and why.

Fixes #123"
```

### 6. Submit a Pull Request

Push your branch and open a PR on GitHub.

---

## Coding Standards

### Rust Style Guide

1. **Formatting**: Use `rustfmt` (configured in rustfmt.toml)
2. **Linting**: All code must pass `clippy` with no warnings
3. **Documentation**: Document all public items with `///`
4. **Unsafe Code**: Minimize and document all `unsafe` blocks

### Code Organization

```rust
//! Module-level documentation explaining purpose
//!
//! # Examples
//!
//! ```rust
//! // Example code here
//! ```

use core::fmt;

/// Struct documentation
///
/// # Safety
///
/// Explain any safety requirements if applicable
#[derive(Debug, Clone)]
pub struct MyStruct {
    /// Field documentation
    field: u32,
}

impl MyStruct {
    /// Method documentation
    pub fn new() -> Self {
        Self { field: 0 }
    }
}
```

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new feature
fix: fix a bug
docs: update documentation
style: format code (no logic changes)
refactor: refactor code (no behavior changes)
test: add or update tests
chore: maintenance tasks
security: security-related changes
```

### Security Requirements

1. **No new unsafe blocks** without explicit justification
2. **Input validation** on all external data
3. **Capability checks** before resource access
4. **No telemetry** or data collection without consent
5. **AI/ML models** must have adversarial robustness testing

---

## AI/ML Contribution Guidelines

### Adding New Models

1. **Define clear inputs/outputs** with fixed sizes
2. **Document training methodology** if applicable
3. **Include adversarial testing**
4. **Benchmark performance impact**
5. **Ensure privacy compliance**

### Model Security Checklist

- [ ] Input validation implemented
- [ ] Output sanitization applied
- [ ] No external network calls
- [ ] No persistent storage without encryption
- [ ] Adversarial examples tested
- [ ] Performance overhead acceptable (< 10μs)

---

## Testing Requirements

### Unit Tests

All new functionality must include unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let result = my_function();
        assert_eq!(result, expected_value);
    }

    #[test]
    #[should_panic(expected = "error message")]
    fn test_error_case() {
        panic_function();
    }
}
```

### Integration Tests

For significant features, add integration tests in `tests/` directory.

### Security Tests

- Fuzz testing for parsers and input handlers
- Property-based testing for critical algorithms
- Static analysis with `cargo-audit`

---

## Documentation

### README Updates

Update README.md if your change affects:
- Installation process
- Configuration options
- API usage
- Features or capabilities

### API Documentation

All public APIs must be documented with:
- Purpose and behavior
- Parameters and return values
- Error conditions
- Safety requirements (if unsafe)
- Examples (for complex APIs)

### Changelog

Add an entry to CHANGELOG.md following this format:

```markdown
## [Unreleased]

### Added
- New feature description (#PR_NUMBER)

### Changed
- Modified behavior description (#PR_NUMBER)

### Fixed
- Bug fix description (#PR_NUMBER)

### Security
- Security improvement description (#PR_NUMBER)
```

---

## Review Process

### What Reviewers Look For

1. **Correctness**: Does the code work as intended?
2. **Safety**: Are there any safety issues?
3. **Performance**: Is the performance acceptable?
4. **Style**: Does it follow coding standards?
5. **Tests**: Are there adequate tests?
6. **Documentation**: Is it properly documented?

### Review Timeline

- Initial review within 5 business days
- Additional reviews as needed
- Merge after approval from at least one maintainer

### Addressing Feedback

- Respond to all comments
- Make requested changes or explain why not
- Re-request review after addressing feedback

---

## Legal Requirements

### Contributor License Agreement

By submitting a pull request, you agree to the following:

1. You grant KeefeOS Team a license to use your contribution under MIT/Apache-2.0
2. Your contribution is your original work or you have rights to submit it
3. You understand your contribution may be distributed under the project's license

### DCO (Developer Certificate of Origin)

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part by
    me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

Sign off your commits with:
```bash
git commit -s -m "feat: your commit message"
```

---

## Getting Help

- **General Questions**: GitHub Discussions
- **Bug Reports**: GitHub Issues
- **Security Issues**: security@keefeos.org
- **Code of Conduct**: conduct@keefeos.org
- **Legal Questions**: legal@keefeos.org

---

## Recognition

Contributors are recognized in:
- CONTRIBUTORS.md file
- Release notes for significant contributions
- Annual contributor spotlight blog posts

Thank you for contributing to KeefeOS! 🎉

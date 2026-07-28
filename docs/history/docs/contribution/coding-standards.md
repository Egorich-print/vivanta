# Coding Standards

These standards ensure consistency and maintainability across the codebase.

*   **Language**: Primarily Rust. Adhere to Rust's idiomatic practices and style guidelines.
*   **Formatting**: Use `rustfmt` for code formatting. This should be integrated into the pre-commit hooks.
    ```bash
    # Example command (to be run locally or in CI)
    cargo fmt --check
    ```
*   **Linting**: Use `clippy` for linting.
    ```bash
    # Example command (to be run locally or in CI)
    cargo clippy -- -D warnings
    ```
*   **Naming Conventions**:
    *   `snake_case` for variables, functions, and modules.
    *   `PascalCase` for structs, enums, and traits.
    *   `SCREAMING_SNAKE_CASE` for constants.
*   **Error Handling**: Utilize Rust's `Result` and `Option` types effectively. Avoid using `unwrap()` or `expect()` in production code unless absolutely necessary and well-justified.
*   **Documentation Comments**: Write clear documentation comments (`///`) for public APIs.
*   **Dependencies**: Keep dependencies minimal and up-to-date. Prefer well-maintained and secure crates.

## General Guidelines

*   Write clear, concise, and readable code.
*   Avoid unnecessary complexity.
*   Prefer immutability where possible.
*   Focus on safety and correctness.

## Specific Technologies

*   **Rust**: Follow the [Rustonomicon](https://doc.rust-lang.org/nomicon/) for low-level details, and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for API design.
*   **C Compatibility Layer**: If C code is necessary, adhere to standard C best practices and ensure strict interface definitions.

## Status: Draft / Final

## Research Needed

*   Determine the exact `rustfmt` and `clippy` versions to be enforced.
*   Establish a policy for introducing new dependencies.

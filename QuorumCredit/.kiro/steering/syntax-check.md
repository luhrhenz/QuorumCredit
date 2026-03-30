# Syntax Check and Code Quality Guidelines

## Always Run Syntax Checks

Before and after implementing any new functionality or making changes to the codebase, you MUST:

1. **Run `cargo check`** to verify syntax and compilation
2. **Run `cargo clippy`** to catch common mistakes and improve code quality
3. **Run `cargo test`** to ensure existing functionality still works

## Code Structure Guidelines

### Modular Organization
- Keep functions organized in logical modules:
  - `types.rs` - Data structures, enums, constants
  - `errors.rs` - Error definitions
  - `helpers.rs` - Utility functions
  - `vouch.rs` - Vouching functionality
  - `loan.rs` - Loan management
  - `admin.rs` - Administrative functions
  - `reputation.rs` - Reputation system integration

### Function Guidelines
- Each function should have a single responsibility
- Use descriptive names for functions and variables
- Add proper error handling with `Result<T, ContractError>`
- Include documentation comments for public functions

### Error Handling
- Always use proper error types from `errors.rs`
- Handle all possible error cases
- Use `assert!` for invariants that should never be violated
- Use `Result` for recoverable errors

## Pre-Implementation Checklist

Before adding any new feature:

1. ✅ Run `cargo check` - must pass without errors
2. ✅ Run `cargo clippy` - address all warnings
3. ✅ Plan the module structure - which file should contain the new code?
4. ✅ Define error cases - what can go wrong?
5. ✅ Write the function signature first
6. ✅ Implement the logic
7. ✅ Test the implementation
8. ✅ Run final syntax check

## Common Syntax Issues to Avoid

- Missing closing braces `}`
- Incorrect function signatures
- Undefined variables or types
- Missing imports
- Incorrect error handling
- Unused variables (use `_` prefix if intentional)

## Testing Requirements

- Write unit tests for new functionality
- Test error cases
- Test edge cases
- Ensure all tests pass before committing

## Commands to Run

```bash
# Check syntax and compilation
cargo check

# Check for common mistakes and style issues
cargo clippy

# Run all tests
cargo test

# Format code
cargo fmt

# Build optimized version
cargo build --release
```

Remember: **Never commit code that doesn't pass `cargo check`**
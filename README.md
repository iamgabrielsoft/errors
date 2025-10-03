# Errors

A Rust library for handling string interpolation in error messages, with support for both named and positional placeholders.


## Features

- **Named Placeholders**: Use `{name}` for named field interpolation
- **Positional Placeholders**: Use `{}` or `{0}`, `{1}`, etc. for positional arguments
- **Format Specifiers**: Supports all standard Rust format specifiers like `:?`, `:x}`, etc.
- **Efficient**: Uses `BTreeSet` for efficient identifier tracking
- **No Panic**: Gracefully handles malformed format strings

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
errors = { git = "https://github.com/iamgabrielsoft/errors.git" }
```

```rust
use errors::parse_internal;

// Basic usage
let (formatted, identifiers) = parse_internal("Hello, {name}! You are {age} years old.");
println!("Formatted: {}", formatted);
println!("Identifiers: {:?}", identifiers);

// Positional placeholders
let (formatted, _) = parse_internal("Hello, {}! Your ID is {:04x}");
println!("{}", formatted); // "Hello, __0! Your ID is __1:04x"
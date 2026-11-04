# some-validation

[![Crates.io](https://img.shields.io/crates/v/some-validation.svg)](https://crates.io/crates/some-validation)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A sovereign, domain-agnostic, zero-I/O abstract validation framework for Rust, featuring composable validation pipelines, RFC-compliant primitives, worldwide telecom prefixes, and optional transport adapters.

## Features

- **Composable Pipelines**:
  - `ValidationPipeline<T>`: Base trait for declarative validation logic.
  - `StringValidationPipeline`: Length constraints, regex matching, sanitization.
  - `NumericValidationPipeline<N>`: Bounds checking, positive/negative constraints, scale enforcement.
  - `BytesValidationPipeline`: Size limits, entropy, format headers.
  - `CollectionValidationPipeline<T>`: Cardinality, uniqueness, element-wise validation.
- **Standards-Compliant Primitives**:
  - Entity Identifiers (`validate_id`)
  - Usernames (`validate_username`)
  - RFC 5322 Emails (`validate_email`)
  - ITU-T E.164 Phone Numbers (`validate_phone_number`)
  - ISO-4217 Currencies (`validate_currency`)
  - ISO-3166-1 Alpha-2 / Alpha-3 Country Codes (`validate_country_code`)
  - UUIDv4 Validation (`validate_uuid`)
  - Configurable Password Policies (`PasswordConfig`)
- **Centralized Metadata**:
  - Worldwide ITU-T dialing prefixes, Canadian NANP NPA mappings, and ISO metadata.
- **Optional Transport Adapters**:
  - `tonic` feature for automatic gRPC `Status::invalid_argument` conversion.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
some-validation = "0.1.0"
```

Or reference directly from GitHub:

```toml
[dependencies]
some-validation = { git = "https://github.com/some-of-cs/some-validation.git" }
```

## Quick Start

```rust
use some_validation::{validate_id, validate_username, validate_email, validate_phone_number};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_id = validate_id("usr_01H7XZA8", "user_id")?;
    let username = validate_username("kareem_99")?;
    let email = validate_email("investor@example.com")?;
    let phone = validate_phone_number("+201001234567")?;

    println!("All fields validated: {user_id}, {username}, {email}, {phone}");
    Ok(())
}
```

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE)).

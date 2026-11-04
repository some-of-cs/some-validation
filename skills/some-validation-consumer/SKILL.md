---
name: some-validation-consumer
description: >-
  Guide for agents consuming `some-validation` in domain crates (`some-auth`, `some-wallet`,
  `some-ads`, `estatestack-db`, `estatestack-kernel`, etc.). Use when validating passwords,
  tokens, OTPs, domain invariants, database schemas, or gRPC requests using `some-validation`
  policy config structs and datatype validation pipelines.
---

# `some-validation` Consumer Skill (Using Abstract Pipelines & Policy Configs)

Use this skill whenever you need to validate data in a domain crate (`some-auth`, `some-wallet`, `estatestack-db`, `estatestack-kernel`, etc.) using `some-validation`.

## Golden Rules for Consumers

1. **Always Pass the Policy / Config Struct**
   - `some-validation` provides abstract, policy-driven validators and datatype pipelines.
   - Instead of calling hardcoded validation functions, declare or construct a `*Config` policy struct, customize it for your domain/context if needed, and pass `(&config, value)` to the validator or pipeline.
2. **Keep Domain-Specific Validations in Your Domain Crate**
   - Never put `auth`, `financial`, `wallet`, or other domain-specific business rules into `some-validation`.
   - Write domain-specific validators inside your domain crate (e.g., `some-auth/src/validation.rs`, `some-auth/src/password_recovery.rs`, `some-wallet/src/validation.rs`) by composing `some-validation`'s datatype pipelines.

---

## Pattern 1: Validating Passwords with `PasswordConfig`

Declare a `PasswordConfig` (default or customized) and pass `(&config, password)` to `validate_password`:

```rust
use some_validation::{validate_password, PasswordConfig, ValidateWith, Validator};

// 1. Standard policy with default compromised password blacklist
let config = PasswordConfig::default().with_default_compromised_passwords();
validate_password(&config, password)?;

// 2. Custom policy via builder methods
let custom_config = PasswordConfig::new()
    .with_min_length(12)
    .with_max_bytes(128)
    .with_require_uppercase(true)
    .with_require_lowercase(true)
    .with_require_digit(true)
    .with_require_symbol(true)
    .with_default_compromised_passwords();

validate_password(&custom_config, password)?;

// 3. Or via trait / inherent methods
custom_config.validate(password)?;
password.validate_with(&custom_config)?;
```

---

## Pattern 2: Building Domain Token / Entity Configs with Datatype Pipelines

When validating domain-specific structures (such as reset tokens, OTPs, wallet balances, or challenges), create a domain `*Config` struct in your domain crate that builds and executes `some-validation` datatype pipelines:

- `StringValidationPipeline`: `non_empty`, `min_length`, `max_length`, `exact_length`, `bytes_in_range`, `numeric_only`, `alphabetic_only`, `ascii_alphanumeric_or`, `starts_with`, `has_part_count`, `equals_trimmed`, `constant_time_eq`, `one_of`, `not_in_case_insensitive`, etc.
- `NumericValidationPipeline<N>`: `min`, `max`, `in_range`, `greater_than`, `less_than`, `equals`, `not_equals`, `not_expired`, `not_in_future`, etc.
- `BytesValidationPipeline`: `non_empty`, `min_bytes`, `max_bytes`, `bytes_in_range`, `exact_length`, `starts_with`, `constant_time_eq`, etc.
- `CollectionValidationPipeline<T>`: `non_empty`, `min_len`, `max_len`, `len_in_range`, `exact_len`, `all`, `any`, `each`, etc.
- `ValidationPipeline<T>`: Generic pipeline for any custom struct `T`.

### Example: OTP & Reset Token Configs (in `some-auth`)

```rust
use some_validation::{
    BytesValidationPipeline, NumericValidationPipeline, StringValidationPipeline, ValidationError,
};

pub struct OtpTokenConfig {
    pub length: usize,
    pub ttl_secs: i64,
    pub numeric_only: bool,
}

impl OtpTokenConfig {
    pub fn string_pipeline(&self) -> StringValidationPipeline {
        let mut p = StringValidationPipeline::new()
            .non_empty("otp")
            .exact_length(self.length);
        if self.numeric_only {
            p = p.numeric_only();
        }
        p
    }

    pub fn timestamp_pipeline(&self, now_unix: i64) -> NumericValidationPipeline<i64> {
        NumericValidationPipeline::new().not_expired(now_unix)
    }

    pub fn validate_otp_match(
        &self,
        expected_otp: &str,
        submitted_otp: &str,
        expires_at_unix: i64,
        now_unix: i64,
    ) -> Result<(), ValidationError> {
        self.timestamp_pipeline(now_unix).validate(&expires_at_unix)?;
        self.string_pipeline().validate(expected_otp)?;
        self.string_pipeline()
            .constant_time_eq("otp", expected_otp)
            .validate(submitted_otp)
    }
}
```

### Example: Numeric Domain Invariants (in `some-wallet`)

```rust
use some_validation::{NumericValidationPipeline, ValidationError};

pub fn validate_fee_bps(fee_bps: u32) -> Result<(), ValidationError> {
    NumericValidationPipeline::<u32>::new()
        .ensure(
            |&val| val <= 10_000,
            |&val| ValidationError::OutOfBounds {
                field: "trading_fee_bps",
                min: "0".to_string(),
                max: "10000".to_string(),
                actual: val.to_string(),
            },
        )
        .validate(&fee_bps)
}
```

---

## Pattern 3: Implementing `Validate` and `ValidateWith<Ctx>` on Schemas

For database documents or domain entities, implement `some_validation::Validate` (or `ValidateWith<Ctx>` when external policy/config is required), combining general primitives from `some_validation` with domain validators from `some_auth` / `some_wallet`:

```rust
impl some_validation::Validate for WalletAccountDocument {
    fn validate(&self) -> Result<(), some_validation::ValidationError> {
        some_validation::validate_non_empty("user_id", &self.user_id)?;
        some_validation::validate_currency_code(&self.currency)?;
        some_wallet::validate_wallet_balances(self.balance_cents, self.locked_cents)?;
        Ok(())
    }
}
```

---

## Pattern 4: Transport Boundary Adaptation (`tonic` Feature)

At gRPC handler boundaries in `estatestack-kernel`, enable the `tonic` feature on `some-validation` and map `Result<T, ValidationError>` to `Result<T, tonic::Status>` using `some_validation::tonic::validation_status`:

```rust
let user_id = some_validation::tonic::validation_status(
    some_validation::validate_id(&req.user_id, "user_id"),
)?;
```

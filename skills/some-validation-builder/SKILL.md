---
name: some-validation-builder
description: >-
  Architectural rules and implementation workflow for builder agents editing, extending,
  or maintaining the `some-validation` crate. Use whenever modifying `some-validation`
  to enforce that it contains strictly abstract, domain-agnostic validation logic,
  datatype validation pipelines, and policy-injected validation functions where a
  configuration/policy struct is passed alongside the value.
---

# `some-validation` Builder Skill (Crate Maintainer & Author Guide)

Use this skill whenever you are editing, refactoring, or adding new capabilities to the `some-validation` crate.

## Core Architectural Invariants

1. **Strictly Abstract & Domain-Agnostic**
   - `some-validation` MUST only contain abstract validation primitives, generic datatype pipelines, and validation traits.
   - **NEVER** add domain-specific or business-module-specific logic (`auth`, `financial`, `wallet`, `kyc`, `trading`, `ads`, `properties`, etc.) to `some-validation`.
   - Domain-specific validations belong in their respective domain crates (e.g., `some-auth::validation`, `some-wallet::validation`).
   - **Framework Adapters**: Feature-gated framework transport adapters (such as `#[cfg(feature = "tonic")] pub mod tonic;`) are permitted as long as core validation rules remain framework-neutral.

2. **Policy / Config Struct Injection (Mandatory for Configurable Rules)**
   - Never hardcode business policies inside validation functions.
   - For any configurable validation rule, you **must**:
     1. Declare a `*Config` policy struct (with `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]` and `Default`).
     2. Provide fluent `with_*` builder methods on the `*Config` struct.
     3. Expose a `.pipeline(&self)` method on the `*Config` struct that builds the appropriate datatype validation pipeline (`StringValidationPipeline`, `NumericValidationPipeline<N>`, `BytesValidationPipeline`, `CollectionValidationPipeline<T>`, or `ValidationPipeline<T>`).
     4. Accept `(&Config, value)` in the validation function: `pub fn validate_<target>(config: &<Target>Config, value: &T) -> Result<(), ValidationError>`.
     5. Implement `Validator<T>` for `<Target>Config` and `ValidateWith<<Target>Config>` for `T`.

3. **Pipeline-First Implementation**
   - Every validation function in `some-validation` must be built on top of the datatype pipelines in `src/pipeline.rs`:
     - `ValidationPipeline<T>`: Generic pipeline for any type `T` (`step`, `step_if`, `ensure`, `ensure_if`, `add_validator`, `merge`, `map_error`, `validate`, `validate_all`).
     - `StringValidationPipeline`: String combinators (`non_empty`, `min_length`, `max_length`, `length_in_range`, `exact_length`, `min_bytes`, `max_bytes`, `bytes_in_range`, `exact_bytes`, `numeric_only`, `alphabetic_only`, `ascii_alphanumeric_only`, `ascii_alphanumeric_or`, `no_whitespace`, `no_control_chars`, `require_uppercase`, `require_lowercase`, `require_digit`, `require_symbol`, `starts_with`, `ends_with`, `contains`, `does_not_start_with`, `does_not_end_with`, `does_not_contain`, `has_part_count`, `min_part_count`, `equals_trimmed`, `constant_time_eq`, `one_of`, `not_in_case_insensitive`, `matches_pattern`).
     - `NumericValidationPipeline<N>`: Numeric combinators (`min`, `max`, `greater_than`, `less_than`, `in_range`, `exclusive_range`, `equals`, `not_equals`, `not_expired`, `not_in_future`).
     - `BytesValidationPipeline`: Byte-slice combinators (`non_empty`, `min_bytes`, `max_bytes`, `bytes_in_range`, `exact_length`, `starts_with`, `constant_time_eq`).
     - `CollectionValidationPipeline<T>`: Slice/collection combinators (`non_empty`, `min_len`, `max_len`, `len_in_range`, `exact_len`, `all`, `any`, `each`).

4. **Zero I/O**
   - `some-validation` must remain 100% synchronous, deterministic, and zero-I/O (no network, database, Redis, or filesystem calls).

---

## Step-by-Step Workflow for Adding a New Validator to `some-validation`

### Step 1: Check Domain Boundary
Ask: *Is this validation specific to a single business domain (e.g., Auth, Wallet, KYC, Trading)?*
- **Yes** -> **STOP.** Do not add it to `some-validation`. Add it to that domain's crate (e.g., `some-auth/src/validation.rs` or `some-wallet/src/validation.rs`) using `some-validation` pipelines (see the `some-validation-consumer` skill).
- **No (it is an abstract datatype combinator or general primitive)** -> Proceed to Step 2.

### Step 2: Add Combinators or Policy Config Struct
If adding a general datatype check, add a combinator method to the corresponding pipeline in `src/pipeline.rs`.

If adding a configurable abstract validator in `src/primitives.rs`, follow this canonical template:

```rust
use crate::error::ValidationError;
use crate::pipeline::StringValidationPipeline;
use crate::traits::{ValidateWith, Validator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MyTargetConfig {
    pub min_length: usize,
    pub max_bytes: usize,
    pub disallow_control_chars: bool,
}

impl Default for MyTargetConfig {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_bytes: 128,
            disallow_control_chars: true,
        }
    }
}

impl MyTargetConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = min_length;
        self
    }

    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// Builds the underlying datatype validation pipeline from this policy config.
    pub fn pipeline(&self) -> StringValidationPipeline {
        let min = self.min_length;
        let max = self.max_bytes;

        StringValidationPipeline::new()
            .ensure_if(
                min > 0,
                move |s| s.chars().count() >= min,
                move |s| ValidationError::OutOfBounds {
                    field: "my_target",
                    min: min.to_string(),
                    max: max.to_string(),
                    actual: s.chars().count().to_string(),
                },
            )
            .ensure_if(
                max > 0,
                move |s| s.len() <= max,
                move |s| ValidationError::OutOfBounds {
                    field: "my_target",
                    min: min.to_string(),
                    max: max.to_string(),
                    actual: s.len().to_string(),
                },
            )
            .step_if(self.disallow_control_chars, |s| {
                StringValidationPipeline::new().no_control_chars().validate(s)
            })
    }

    pub fn validate(&self, value: &str) -> Result<(), ValidationError> {
        self.pipeline().validate(value)
    }
}

/// Validates `value` against the provided `config` policy.
pub fn validate_my_target(config: &MyTargetConfig, value: &str) -> Result<(), ValidationError> {
    config.pipeline().validate(value)
}

impl Validator<str> for MyTargetConfig {
    fn validate(&self, target: &str) -> Result<(), ValidationError> {
        validate_my_target(self, target)
    }
}

impl ValidateWith<MyTargetConfig> for str {
    fn validate_with(&self, ctx: &MyTargetConfig) -> Result<(), ValidationError> {
        validate_my_target(ctx, self)
    }
}
```

### Step 3: Verify
Run the `some-validation` unit and doc tests:
```bash
cargo test -p some-validation
```

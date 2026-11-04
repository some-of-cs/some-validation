//! # Abstract Datatype Validation Pipelines
//!
//! Provides composable, reusable, zero-I/O validation pipelines for standard datatypes:
//! - [`ValidationPipeline<T>`]: Universal abstract validation pipeline for any target type `T`.
//! - [`StringValidationPipeline`]: Specialized validation pipeline for string/text data types (`str`).
//! - [`NumericValidationPipeline<N>`]: Specialized validation pipeline for numeric data types (`i64`, `u64`, `usize`, `f64`, etc.).
//! - [`BytesValidationPipeline`]: Specialized validation pipeline for raw byte slices (`[u8]`).
//! - [`CollectionValidationPipeline<T>`]: Specialized validation pipeline for slices and collections (`[T]`).

use crate::error::ValidationError;

/// Performs a constant-time byte slice comparison to prevent timing side-channel attacks.
pub fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ============================================================================
// Universal Generic Validation Pipeline
// ============================================================================

/// A boxed validation step closure that evaluates an invariant against `&T`.
pub type ValidationStep<T> = Box<dyn Fn(&T) -> Result<(), ValidationError> + Send + Sync>;

/// Abstract validation pipeline for any target type `T`.
/// Chains validation stages sequentially and supports both fail-fast (`validate`)
/// and error-accumulating (`validate_all`) execution.
pub struct ValidationPipeline<T: ?Sized> {
    steps: Vec<ValidationStep<T>>,
}

impl<T: ?Sized> Default for ValidationPipeline<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> ValidationPipeline<T> {
    /// Creates an empty validation pipeline.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Appends a validation closure to the pipeline.
    pub fn step<F>(mut self, step_fn: F) -> Self
    where
        F: Fn(&T) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.steps.push(Box::new(step_fn));
        self
    }

    /// Conditionally appends a validation closure if `condition` is true.
    pub fn step_if<F>(self, condition: bool, step_fn: F) -> Self
    where
        F: Fn(&T) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        if condition {
            self.step(step_fn)
        } else {
            self
        }
    }

    /// Appends a predicate check that produces `err_fn(target)` when `predicate(target)` returns `false`.
    pub fn ensure<P, E>(self, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
        E: Fn(&T) -> ValidationError + Send + Sync + 'static,
    {
        self.step(move |target| {
            if predicate(target) {
                Ok(())
            } else {
                Err(err_fn(target))
            }
        })
    }

    /// Conditionally appends a predicate check if `condition` is true.
    pub fn ensure_if<P, E>(self, condition: bool, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
        E: Fn(&T) -> ValidationError + Send + Sync + 'static,
    {
        if condition {
            self.ensure(predicate, err_fn)
        } else {
            self
        }
    }

    /// Appends an abstract [`crate::traits::Validator`] strategy to the pipeline.
    pub fn add_validator<V>(self, validator: V) -> Self
    where
        V: crate::traits::Validator<T> + Send + Sync + 'static,
    {
        self.step(move |target| validator.validate(target))
    }

    /// Merges all steps from `other` into this pipeline.
    pub fn merge(mut self, mut other: Self) -> Self {
        self.steps.append(&mut other.steps);
        self
    }

    /// Transforms any [`ValidationError`] emitted by existing steps in this pipeline using `mapper`.
    pub fn map_error<M>(self, mapper: M) -> Self
    where
        T: 'static,
        M: Fn(ValidationError) -> ValidationError + Send + Sync + 'static,
    {
        let prior_steps = self.steps;
        let mut mapped = Self::new();
        mapped.steps.push(Box::new(move |target| {
            for step in &prior_steps {
                if let Err(e) = step(target) {
                    return Err(mapper(e));
                }
            }
            Ok(())
        }));
        mapped
    }

    /// Executes all pipeline stages sequentially against `target`, failing fast on the first error.
    pub fn validate(&self, target: &T) -> Result<(), ValidationError> {
        for step in &self.steps {
            step(target)?;
        }
        Ok(())
    }

    /// Executes all pipeline stages against `target`, collecting all errors that occur.
    pub fn validate_all(&self, target: &T) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        for step in &self.steps {
            if let Err(err) = step(target) {
                errors.push(err);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl<T: ?Sized> crate::traits::Validator<T> for ValidationPipeline<T> {
    fn validate(&self, target: &T) -> Result<(), ValidationError> {
        self.validate(target)
    }
}

// ============================================================================
// Specialized String Validation Pipeline
// ============================================================================

/// Abstract validation pipeline tailored for string and text data types (`str`).
#[derive(Default)]
pub struct StringValidationPipeline {
    pipeline: ValidationPipeline<str>,
}

impl StringValidationPipeline {
    /// Creates an empty string validation pipeline.
    pub fn new() -> Self {
        Self {
            pipeline: ValidationPipeline::new(),
        }
    }

    /// Appends a custom validation closure to the string pipeline.
    pub fn step<F>(mut self, step_fn: F) -> Self
    where
        F: Fn(&str) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step(step_fn);
        self
    }

    /// Conditionally appends a validation closure if `condition` is true.
    pub fn step_if<F>(mut self, condition: bool, step_fn: F) -> Self
    where
        F: Fn(&str) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step_if(condition, step_fn);
        self
    }

    /// Appends a predicate check that emits `err_fn(s)` when `predicate(s)` returns `false`.
    pub fn ensure<P, E>(mut self, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&str) -> bool + Send + Sync + 'static,
        E: Fn(&str) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure(predicate, err_fn);
        self
    }

    /// Conditionally appends a predicate check if `condition` is true.
    pub fn ensure_if<P, E>(mut self, condition: bool, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&str) -> bool + Send + Sync + 'static,
        E: Fn(&str) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure_if(condition, predicate, err_fn);
        self
    }

    /// Merges another [`StringValidationPipeline`] into this pipeline.
    pub fn merge(mut self, other: Self) -> Self {
        self.pipeline = self.pipeline.merge(other.pipeline);
        self
    }

    /// Transforms any [`ValidationError`] produced by the current steps using `mapper`.
    pub fn map_error<M>(mut self, mapper: M) -> Self
    where
        M: Fn(ValidationError) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.map_error(mapper);
        self
    }

    /// Requires the string to be non-empty after trimming whitespace.
    pub fn non_empty(self, field: &'static str) -> Self {
        self.ensure(
            |s| !s.trim().is_empty(),
            move |_| ValidationError::FieldRequired(field.to_string()),
        )
    }

    /// Enforces a minimum Unicode scalar character length.
    pub fn min_length(self, min: usize) -> Self {
        self.ensure(
            move |s| s.chars().count() >= min,
            move |s| ValidationError::OutOfBounds {
                field: "string_length",
                min: min.to_string(),
                max: "unbounded".to_string(),
                actual: s.chars().count().to_string(),
            },
        )
    }

    /// Enforces a maximum Unicode scalar character length.
    pub fn max_length(self, max: usize) -> Self {
        self.ensure(
            move |s| s.chars().count() <= max,
            move |s| ValidationError::OutOfBounds {
                field: "string_length",
                min: "0".to_string(),
                max: max.to_string(),
                actual: s.chars().count().to_string(),
            },
        )
    }

    /// Enforces that Unicode scalar character count falls within `[min, max]`.
    pub fn length_in_range(self, min: usize, max: usize) -> Self {
        self.ensure(
            move |s| {
                let count = s.chars().count();
                count >= min && count <= max
            },
            move |s| ValidationError::OutOfBounds {
                field: "string_length",
                min: min.to_string(),
                max: max.to_string(),
                actual: s.chars().count().to_string(),
            },
        )
    }

    /// Enforces an exact trimmed character length.
    pub fn exact_length(self, exact: usize) -> Self {
        self.ensure(
            move |s| s.trim().chars().count() == exact,
            move |s| ValidationError::OutOfBounds {
                field: "string_exact_length",
                min: exact.to_string(),
                max: exact.to_string(),
                actual: s.trim().chars().count().to_string(),
            },
        )
    }

    /// Enforces a minimum UTF-8 byte length.
    pub fn min_bytes(self, min: usize) -> Self {
        self.ensure(
            move |s| s.len() >= min,
            move |s| ValidationError::OutOfBounds {
                field: "string_bytes",
                min: min.to_string(),
                max: "unbounded".to_string(),
                actual: s.len().to_string(),
            },
        )
    }

    /// Enforces a maximum UTF-8 byte length.
    pub fn max_bytes(self, max: usize) -> Self {
        self.ensure(
            move |s| s.len() <= max,
            move |s| ValidationError::OutOfBounds {
                field: "string_bytes",
                min: "0".to_string(),
                max: max.to_string(),
                actual: s.len().to_string(),
            },
        )
    }

    /// Enforces that UTF-8 byte length falls within `[min, max]`.
    pub fn bytes_in_range(self, field: &'static str, min: usize, max: usize) -> Self {
        self.ensure(
            move |s| s.len() >= min && s.len() <= max,
            move |s| ValidationError::OutOfBounds {
                field,
                min: min.to_string(),
                max: max.to_string(),
                actual: s.len().to_string(),
            },
        )
    }

    /// Enforces an exact UTF-8 byte length.
    pub fn exact_bytes(self, exact: usize) -> Self {
        self.ensure(
            move |s| s.len() == exact,
            move |s| ValidationError::OutOfBounds {
                field: "string_exact_bytes",
                min: exact.to_string(),
                max: exact.to_string(),
                actual: s.len().to_string(),
            },
        )
    }

    /// Requires the trimmed string to contain only ASCII digits.
    pub fn numeric_only(self) -> Self {
        self.ensure(
            |s| {
                let t = s.trim();
                !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
            },
            |_| ValidationError::InvalidFormat {
                field: "numeric_string",
                message: "String must contain ASCII digits only".to_string(),
            },
        )
    }

    /// Requires the trimmed string to contain only ASCII alphabetic characters.
    pub fn alphabetic_only(self) -> Self {
        self.ensure(
            |s| {
                let t = s.trim();
                !t.is_empty() && t.chars().all(|c| c.is_ascii_alphabetic())
            },
            |_| ValidationError::InvalidFormat {
                field: "alphabetic_string",
                message: "String must contain ASCII alphabetic characters only".to_string(),
            },
        )
    }

    /// Requires the trimmed string to contain only ASCII alphanumeric characters.
    pub fn ascii_alphanumeric_only(self) -> Self {
        self.ensure(
            |s| {
                let t = s.trim();
                !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric())
            },
            |_| ValidationError::InvalidFormat {
                field: "alphanumeric_string",
                message: "String must contain ASCII alphanumeric characters only".to_string(),
            },
        )
    }

    /// Requires the trimmed string to contain only ASCII hexadecimal characters (`0-9`, `a-f`, `A-F`).
    pub fn hex_only(self) -> Self {
        self.ensure(
            |s| {
                let t = s.trim();
                !t.is_empty() && t.chars().all(|c| c.is_ascii_hexdigit())
            },
            |_| ValidationError::InvalidFormat {
                field: "hex_string",
                message: "String must contain ASCII hexadecimal characters only".to_string(),
            },
        )
    }

    /// Requires the trimmed string to contain only RFC 4648 Base32 characters (`A-Z`, `a-z`, `2-7`, optional `=`).
    pub fn base32_only(self) -> Self {
        self.ensure(
            |s| {
                let t = s.trim();
                !t.is_empty()
                    && t.chars()
                        .all(|c| matches!(c.to_ascii_uppercase(), 'A'..='Z' | '2'..='7' | '='))
            },
            |_| ValidationError::InvalidFormat {
                field: "base32_string",
                message: "String must contain valid RFC 4648 Base32 characters only".to_string(),
            },
        )
    }

    /// Requires the trimmed string to be a valid RFC 4122 UUID.
    pub fn uuid(self) -> Self {
        self.ensure(
            |s| uuid::Uuid::parse_str(s.trim()).is_ok(),
            |s| ValidationError::InvalidFormat {
                field: "uuid",
                message: format!("'{}' is not a valid UUID", s),
            },
        )
    }

    /// Requires the trimmed string to start with `prefix` followed by a valid RFC 4122 UUID.
    pub fn prefixed_uuid(self, prefix: &'static str) -> Self {
        self.ensure(
            move |s| {
                s.trim()
                    .strip_prefix(prefix)
                    .is_some_and(|rest| uuid::Uuid::parse_str(rest).is_ok())
            },
            move |s| ValidationError::InvalidFormat {
                field: "prefixed_uuid",
                message: format!(
                    "'{}' must start with '{}' followed by a valid UUID",
                    s, prefix
                ),
            },
        )
    }

    /// Requires the trimmed string to be a 24-character hexadecimal MongoDB ObjectId.
    pub fn mongodb_object_id(self) -> Self {
        self.exact_length(24)
            .hex_only()
            .map_error(|_| ValidationError::InvalidFormat {
                field: "mongodb_id",
                message: "Value must be a 24-character hexadecimal MongoDB ObjectId".to_string(),
            })
    }

    /// Requires all characters to be ASCII alphanumeric or in `allowed_extra`.
    pub fn ascii_alphanumeric_or(self, allowed_extra: &'static [char]) -> Self {
        self.ensure(
            move |s| {
                !s.is_empty()
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || allowed_extra.contains(&c))
            },
            |_| ValidationError::InvalidFormat {
                field: "alphanumeric_custom",
                message: "String contains disallowed characters".to_string(),
            },
        )
    }

    /// Prohibits any whitespace characters.
    pub fn no_whitespace(self) -> Self {
        self.ensure(
            |s| !s.chars().any(|c| c.is_whitespace()),
            |_| ValidationError::InvalidFormat {
                field: "whitespace",
                message: "String must not contain whitespace".to_string(),
            },
        )
    }

    /// Prohibits any ASCII/Unicode control characters.
    pub fn no_control_chars(self) -> Self {
        self.ensure(
            |s| !s.chars().any(|c| c.is_control()),
            |_| ValidationError::InvalidFormat {
                field: "control_chars",
                message: "String must not contain control characters".to_string(),
            },
        )
    }

    /// Requires at least one ASCII uppercase letter.
    pub fn require_uppercase(self) -> Self {
        self.ensure(
            |s| s.chars().any(|c| c.is_ascii_uppercase()),
            |_| ValidationError::InvalidFormat {
                field: "uppercase",
                message: "String must contain at least one uppercase letter".to_string(),
            },
        )
    }

    /// Requires at least one ASCII lowercase letter.
    pub fn require_lowercase(self) -> Self {
        self.ensure(
            |s| s.chars().any(|c| c.is_ascii_lowercase()),
            |_| ValidationError::InvalidFormat {
                field: "lowercase",
                message: "String must contain at least one lowercase letter".to_string(),
            },
        )
    }

    /// Requires at least one ASCII digit.
    pub fn require_digit(self) -> Self {
        self.ensure(
            |s| s.chars().any(|c| c.is_ascii_digit()),
            |_| ValidationError::InvalidFormat {
                field: "digit",
                message: "String must contain at least one digit".to_string(),
            },
        )
    }

    /// Requires at least one special symbol or punctuation character (excluding whitespace and control chars).
    pub fn require_symbol(self) -> Self {
        self.ensure(
            |s| {
                s.chars().any(|c| {
                    c.is_ascii_punctuation()
                        || (!c.is_alphanumeric() && !c.is_whitespace() && !c.is_control())
                })
            },
            |_| ValidationError::InvalidFormat {
                field: "symbol",
                message: "String must contain at least one special symbol".to_string(),
            },
        )
    }

    /// Requires the string to start with `prefix`.
    pub fn starts_with(self, prefix: impl Into<String>) -> Self {
        let p = prefix.into();
        self.ensure(
            move |s| s.starts_with(&p),
            move |_| ValidationError::InvalidFormat {
                field: "prefix",
                message: "String does not start with expected prefix".to_string(),
            },
        )
    }

    /// Requires the string to end with `suffix`.
    pub fn ends_with(self, suffix: impl Into<String>) -> Self {
        let sfx = suffix.into();
        self.ensure(
            move |s| s.ends_with(&sfx),
            move |_| ValidationError::InvalidFormat {
                field: "suffix",
                message: "String does not end with expected suffix".to_string(),
            },
        )
    }

    /// Requires the string to contain `needle`.
    pub fn contains(self, needle: impl Into<String>) -> Self {
        let n = needle.into();
        self.ensure(
            move |s| s.contains(&n),
            move |_| ValidationError::InvalidFormat {
                field: "contains",
                message: "String does not contain required substring".to_string(),
            },
        )
    }

    /// Prohibits the string from starting with `prefix`.
    pub fn does_not_start_with(self, prefix: impl Into<String>) -> Self {
        let p = prefix.into();
        self.ensure(
            move |s| !s.starts_with(&p),
            move |_| ValidationError::InvalidFormat {
                field: "prefix",
                message: "String starts with prohibited prefix".to_string(),
            },
        )
    }

    /// Prohibits the string from ending with `suffix`.
    pub fn does_not_end_with(self, suffix: impl Into<String>) -> Self {
        let sfx = suffix.into();
        self.ensure(
            move |s| !s.ends_with(&sfx),
            move |_| ValidationError::InvalidFormat {
                field: "suffix",
                message: "String ends with prohibited suffix".to_string(),
            },
        )
    }

    /// Prohibits the string from containing `needle`.
    pub fn does_not_contain(self, needle: impl Into<String>) -> Self {
        let n = needle.into();
        self.ensure(
            move |s| !s.contains(&n),
            move |_| ValidationError::InvalidFormat {
                field: "contains",
                message: "String contains prohibited substring".to_string(),
            },
        )
    }

    /// Requires the string when split by `delimiter` to have exactly `count` parts.
    pub fn has_part_count(self, delimiter: char, count: usize) -> Self {
        self.ensure(
            move |s| s.split(delimiter).count() == count,
            move |s| ValidationError::InvalidFormat {
                field: "part_count",
                message: format!(
                    "Expected {} parts delimited by '{}', got {}",
                    count,
                    delimiter,
                    s.split(delimiter).count()
                ),
            },
        )
    }

    /// Requires the string when split by `delimiter` to have at least `min_count` parts.
    pub fn min_part_count(self, delimiter: char, min_count: usize) -> Self {
        self.ensure(
            move |s| s.split(delimiter).count() >= min_count,
            move |s| ValidationError::InvalidFormat {
                field: "part_count",
                message: format!(
                    "Expected at least {} parts delimited by '{}', got {}",
                    min_count,
                    delimiter,
                    s.split(delimiter).count()
                ),
            },
        )
    }

    /// Requires exact equality with `expected` (after trimming).
    pub fn equals_trimmed(self, field: &'static str, expected: impl Into<String>) -> Self {
        let exp = expected.into();
        let exp_trimmed = exp.trim().to_string();
        self.ensure(
            move |s| s.trim() == exp_trimmed,
            move |_| ValidationError::InvalidFormat {
                field,
                message: format!("Value for '{}' does not match expected", field),
            },
        )
    }

    /// Requires constant-time byte equality with `expected` (after trimming).
    pub fn constant_time_eq(self, field: &'static str, expected: impl Into<String>) -> Self {
        let exp = expected.into().trim().to_string();
        self.ensure(
            move |s| constant_time_eq_bytes(s.trim().as_bytes(), exp.as_bytes()),
            move |_| ValidationError::InvalidFormat {
                field,
                message: format!("Verification failed for '{}'", field),
            },
        )
    }

    /// Requires the string to match one of the allowed values.
    pub fn one_of<I, S>(self, field: &'static str, allowed: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let set: Vec<String> = allowed.into_iter().map(Into::into).collect();
        self.ensure(
            move |s| set.iter().any(|item| item == s),
            move |s| ValidationError::InvalidFormat {
                field,
                message: format!("'{}' is not an allowed value for '{}'", s, field),
            },
        )
    }

    /// Prohibits the string from matching any item in `disallowed` (case-insensitive).
    pub fn not_in_case_insensitive<I, S>(self, field: &'static str, disallowed: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list: Vec<String> = disallowed
            .into_iter()
            .map(|s| s.into().to_lowercase())
            .collect();
        self.ensure(
            move |s| {
                let lower = s.to_lowercase();
                !list.iter().any(|d| d == &lower)
            },
            move |_| ValidationError::InvalidFormat {
                field,
                message: format!("Value for '{}' is disallowed or reserved", field),
            },
        )
    }

    /// Enforces a generic predicate condition on the string.
    pub fn matches_pattern<P>(self, predicate: P, err_message: &'static str) -> Self
    where
        P: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.ensure(predicate, move |_| ValidationError::InvalidFormat {
            field: "pattern",
            message: err_message.to_string(),
        })
    }

    /// Validates the given string against all pipeline stages (fail-fast).
    pub fn validate(&self, s: &str) -> Result<(), ValidationError> {
        self.pipeline.validate(s)
    }

    /// Validates the given string against all pipeline stages, collecting all errors.
    pub fn validate_all(&self, s: &str) -> Result<(), Vec<ValidationError>> {
        self.pipeline.validate_all(s)
    }
}

impl crate::traits::Validator<str> for StringValidationPipeline {
    fn validate(&self, target: &str) -> Result<(), ValidationError> {
        self.validate(target)
    }
}

// ============================================================================
// Specialized Numeric Validation Pipeline
// ============================================================================

/// Abstract validation pipeline tailored for numeric types (`i64`, `u64`, `usize`, `f64`, etc.).
pub struct NumericValidationPipeline<N> {
    pipeline: ValidationPipeline<N>,
}

impl<N: 'static> Default for NumericValidationPipeline<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: 'static> NumericValidationPipeline<N> {
    /// Creates an empty numeric validation pipeline.
    pub fn new() -> Self {
        Self {
            pipeline: ValidationPipeline::new(),
        }
    }

    /// Appends a custom validation closure to the numeric pipeline.
    pub fn step<F>(mut self, step_fn: F) -> Self
    where
        F: Fn(&N) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step(step_fn);
        self
    }

    /// Conditionally appends a validation closure if `condition` is true.
    pub fn step_if<F>(mut self, condition: bool, step_fn: F) -> Self
    where
        F: Fn(&N) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step_if(condition, step_fn);
        self
    }

    /// Appends a predicate check that emits `err_fn(val)` when `predicate(val)` returns `false`.
    pub fn ensure<P, E>(mut self, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&N) -> bool + Send + Sync + 'static,
        E: Fn(&N) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure(predicate, err_fn);
        self
    }

    /// Conditionally appends a predicate check if `condition` is true.
    pub fn ensure_if<P, E>(mut self, condition: bool, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&N) -> bool + Send + Sync + 'static,
        E: Fn(&N) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure_if(condition, predicate, err_fn);
        self
    }

    /// Transforms any [`ValidationError`] produced by current steps using `mapper`.
    pub fn map_error<M>(mut self, mapper: M) -> Self
    where
        M: Fn(ValidationError) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.map_error(mapper);
        self
    }

    /// Validates the given numeric reference against all pipeline stages (fail-fast).
    pub fn validate(&self, val: &N) -> Result<(), ValidationError> {
        self.pipeline.validate(val)
    }

    /// Validates the given numeric reference against all pipeline stages, collecting all errors.
    pub fn validate_all(&self, val: &N) -> Result<(), Vec<ValidationError>> {
        self.pipeline.validate_all(val)
    }
}

impl<N: PartialOrd + std::fmt::Display + Copy + Send + Sync + 'static>
    NumericValidationPipeline<N>
{
    /// Requires the number to be greater than or equal to `min_val`.
    pub fn min(self, min_val: N) -> Self {
        self.ensure(
            move |val| *val >= min_val,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: min_val.to_string(),
                max: "unbounded".to_string(),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to be less than or equal to `max_val`.
    pub fn max(self, max_val: N) -> Self {
        self.ensure(
            move |val| *val <= max_val,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: "unbounded".to_string(),
                max: max_val.to_string(),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to be strictly greater than `bound`.
    pub fn greater_than(self, bound: N) -> Self {
        self.ensure(
            move |val| *val > bound,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: format!("> {}", bound),
                max: "unbounded".to_string(),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to be strictly less than `bound`.
    pub fn less_than(self, bound: N) -> Self {
        self.ensure(
            move |val| *val < bound,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: "unbounded".to_string(),
                max: format!("< {}", bound),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to be within inclusive bounds `[min_val, max_val]`.
    pub fn in_range(self, min_val: N, max_val: N) -> Self {
        self.ensure(
            move |val| *val >= min_val && *val <= max_val,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: min_val.to_string(),
                max: max_val.to_string(),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to be within exclusive bounds `(min_val, max_val)`.
    pub fn exclusive_range(self, min_val: N, max_val: N) -> Self {
        self.ensure(
            move |val| *val > min_val && *val < max_val,
            move |val| ValidationError::OutOfBounds {
                field: "numeric",
                min: min_val.to_string(),
                max: max_val.to_string(),
                actual: val.to_string(),
            },
        )
    }

    /// Requires the number to equal `expected`.
    pub fn equals(self, field: &'static str, expected: N) -> Self {
        self.ensure(
            move |val| *val == expected,
            move |val| ValidationError::InvalidFormat {
                field,
                message: format!("Expected {}, got {}", expected, val),
            },
        )
    }

    /// Requires the number not to equal `disallowed`.
    pub fn not_equals(self, field: &'static str, disallowed: N) -> Self {
        self.ensure(
            move |val| *val != disallowed,
            move |val| ValidationError::InvalidFormat {
                field,
                message: format!("Value {} is not allowed for '{}'", val, field),
            },
        )
    }

    /// Validates that an expiration timestamp has not passed relative to `current_time` (`current_time <= expires_at`).
    pub fn not_expired(self, current_time: N) -> Self {
        self.ensure(
            move |expires_at| current_time <= *expires_at,
            |_| ValidationError::ExpiredSession,
        )
    }

    /// Validates that an issuance timestamp is not in the future relative to `current_time` (`issued_at <= current_time`).
    pub fn not_in_future(self, current_time: N) -> Self {
        self.ensure(
            move |issued_at| *issued_at <= current_time,
            |_| ValidationError::InvalidFormat {
                field: "issued_at",
                message: "Timestamp cannot be in the future".to_string(),
            },
        )
    }
}

impl<N: 'static> crate::traits::Validator<N> for NumericValidationPipeline<N> {
    fn validate(&self, target: &N) -> Result<(), ValidationError> {
        self.validate(target)
    }
}

// ============================================================================
// Specialized Bytes Validation Pipeline
// ============================================================================

/// Abstract validation pipeline tailored for raw byte slices (`[u8]`).
#[derive(Default)]
pub struct BytesValidationPipeline {
    pipeline: ValidationPipeline<[u8]>,
}

impl BytesValidationPipeline {
    /// Creates an empty bytes validation pipeline.
    pub fn new() -> Self {
        Self {
            pipeline: ValidationPipeline::new(),
        }
    }

    /// Appends a custom validation closure to the bytes pipeline.
    pub fn step<F>(mut self, step_fn: F) -> Self
    where
        F: Fn(&[u8]) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step(step_fn);
        self
    }

    /// Conditionally appends a validation closure if `condition` is true.
    pub fn step_if<F>(mut self, condition: bool, step_fn: F) -> Self
    where
        F: Fn(&[u8]) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step_if(condition, step_fn);
        self
    }

    /// Appends a predicate check that emits `err_fn(bytes)` when `predicate(bytes)` returns `false`.
    pub fn ensure<P, E>(mut self, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&[u8]) -> bool + Send + Sync + 'static,
        E: Fn(&[u8]) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure(predicate, err_fn);
        self
    }

    /// Conditionally appends a predicate check if `condition` is true.
    pub fn ensure_if<P, E>(mut self, condition: bool, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&[u8]) -> bool + Send + Sync + 'static,
        E: Fn(&[u8]) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure_if(condition, predicate, err_fn);
        self
    }

    /// Transforms any [`ValidationError`] produced by current steps using `mapper`.
    pub fn map_error<M>(mut self, mapper: M) -> Self
    where
        M: Fn(ValidationError) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.map_error(mapper);
        self
    }

    /// Requires the byte slice to be non-empty.
    pub fn non_empty(self, field: &'static str) -> Self {
        self.ensure(
            |b| !b.is_empty(),
            move |_| ValidationError::FieldRequired(field.to_string()),
        )
    }

    /// Enforces a minimum byte length.
    pub fn min_bytes(self, min: usize) -> Self {
        self.ensure(
            move |b| b.len() >= min,
            move |b| ValidationError::OutOfBounds {
                field: "bytes_length",
                min: min.to_string(),
                max: "unbounded".to_string(),
                actual: b.len().to_string(),
            },
        )
    }

    /// Enforces a maximum byte length.
    pub fn max_bytes(self, max: usize) -> Self {
        self.ensure(
            move |b| b.len() <= max,
            move |b| ValidationError::OutOfBounds {
                field: "bytes_length",
                min: "0".to_string(),
                max: max.to_string(),
                actual: b.len().to_string(),
            },
        )
    }

    /// Enforces that byte length falls within `[min, max]`.
    pub fn bytes_in_range(self, min: usize, max: usize) -> Self {
        self.ensure(
            move |b| b.len() >= min && b.len() <= max,
            move |b| ValidationError::OutOfBounds {
                field: "bytes_length",
                min: min.to_string(),
                max: max.to_string(),
                actual: b.len().to_string(),
            },
        )
    }

    /// Enforces an exact byte length.
    pub fn exact_length(self, exact: usize) -> Self {
        self.ensure(
            move |b| b.len() == exact,
            move |b| ValidationError::OutOfBounds {
                field: "bytes_exact_length",
                min: exact.to_string(),
                max: exact.to_string(),
                actual: b.len().to_string(),
            },
        )
    }

    /// Requires the byte slice to start with `prefix`.
    pub fn starts_with(self, prefix: Vec<u8>) -> Self {
        self.ensure(
            move |b| b.starts_with(&prefix),
            |_| ValidationError::InvalidFormat {
                field: "bytes_prefix",
                message: "Byte slice does not start with expected prefix".to_string(),
            },
        )
    }

    /// Requires constant-time byte equality with `expected`.
    pub fn constant_time_eq(self, field: &'static str, expected: Vec<u8>) -> Self {
        self.ensure(
            move |b| constant_time_eq_bytes(b, &expected),
            move |_| ValidationError::InvalidFormat {
                field,
                message: format!("Cryptographic byte comparison failed for '{}'", field),
            },
        )
    }

    /// Validates the given byte slice against all pipeline stages (fail-fast).
    pub fn validate(&self, bytes: &[u8]) -> Result<(), ValidationError> {
        self.pipeline.validate(bytes)
    }

    /// Validates the given byte slice against all pipeline stages, collecting all errors.
    pub fn validate_all(&self, bytes: &[u8]) -> Result<(), Vec<ValidationError>> {
        self.pipeline.validate_all(bytes)
    }
}

impl crate::traits::Validator<[u8]> for BytesValidationPipeline {
    fn validate(&self, target: &[u8]) -> Result<(), ValidationError> {
        self.validate(target)
    }
}

// ============================================================================
// Specialized Collection / Slice Validation Pipeline
// ============================================================================

/// Abstract validation pipeline tailored for slices and collections (`[T]`).
pub struct CollectionValidationPipeline<T> {
    pipeline: ValidationPipeline<[T]>,
}

impl<T: 'static> Default for CollectionValidationPipeline<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> CollectionValidationPipeline<T> {
    /// Creates an empty collection validation pipeline.
    pub fn new() -> Self {
        Self {
            pipeline: ValidationPipeline::new(),
        }
    }

    /// Appends a custom validation closure to the collection pipeline.
    pub fn step<F>(mut self, step_fn: F) -> Self
    where
        F: Fn(&[T]) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step(step_fn);
        self
    }

    /// Conditionally appends a validation closure if `condition` is true.
    pub fn step_if<F>(mut self, condition: bool, step_fn: F) -> Self
    where
        F: Fn(&[T]) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.step_if(condition, step_fn);
        self
    }

    /// Appends a predicate check on the slice.
    pub fn ensure<P, E>(mut self, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&[T]) -> bool + Send + Sync + 'static,
        E: Fn(&[T]) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure(predicate, err_fn);
        self
    }

    /// Conditionally appends a predicate check on the slice if `condition` is true.
    pub fn ensure_if<P, E>(mut self, condition: bool, predicate: P, err_fn: E) -> Self
    where
        P: Fn(&[T]) -> bool + Send + Sync + 'static,
        E: Fn(&[T]) -> ValidationError + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.ensure_if(condition, predicate, err_fn);
        self
    }

    /// Requires the collection to be non-empty.
    pub fn non_empty(self, field: &'static str) -> Self {
        self.ensure(
            |items| !items.is_empty(),
            move |_| ValidationError::FieldRequired(field.to_string()),
        )
    }

    /// Enforces a minimum number of elements.
    pub fn min_len(self, min: usize) -> Self {
        self.ensure(
            move |items| items.len() >= min,
            move |items| ValidationError::OutOfBounds {
                field: "collection_length",
                min: min.to_string(),
                max: "unbounded".to_string(),
                actual: items.len().to_string(),
            },
        )
    }

    /// Enforces a maximum number of elements.
    pub fn max_len(self, max: usize) -> Self {
        self.ensure(
            move |items| items.len() <= max,
            move |items| ValidationError::OutOfBounds {
                field: "collection_length",
                min: "0".to_string(),
                max: max.to_string(),
                actual: items.len().to_string(),
            },
        )
    }

    /// Enforces that element count falls within `[min, max]`.
    pub fn len_in_range(self, min: usize, max: usize) -> Self {
        self.ensure(
            move |items| items.len() >= min && items.len() <= max,
            move |items| ValidationError::OutOfBounds {
                field: "collection_length",
                min: min.to_string(),
                max: max.to_string(),
                actual: items.len().to_string(),
            },
        )
    }

    /// Enforces an exact element count.
    pub fn exact_len(self, exact: usize) -> Self {
        self.ensure(
            move |items| items.len() == exact,
            move |items| ValidationError::OutOfBounds {
                field: "collection_exact_length",
                min: exact.to_string(),
                max: exact.to_string(),
                actual: items.len().to_string(),
            },
        )
    }

    /// Requires distinct borrowed keys without cloning collection items or keys.
    pub fn unique_by<K, F>(self, field: &'static str, key: F) -> Self
    where
        K: Eq + std::hash::Hash + ?Sized + 'static,
        F: for<'a> Fn(&'a T) -> &'a K + Send + Sync + 'static,
    {
        self.ensure(
            move |items| {
                let mut seen = std::collections::HashSet::with_capacity(items.len());
                items.iter().all(|item| seen.insert(key(item)))
            },
            move |_| ValidationError::InvalidFormat {
                field,
                message: "Collection keys must be unique".to_string(),
            },
        )
    }

    /// Requires all elements in the slice to satisfy `predicate`.
    pub fn all<P>(self, predicate: P, err_message: &'static str) -> Self
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.ensure(
            move |items| items.iter().all(&predicate),
            move |_| ValidationError::InvalidFormat {
                field: "collection_items",
                message: err_message.to_string(),
            },
        )
    }

    /// Requires at least one element in the slice to satisfy `predicate`.
    pub fn any<P>(self, predicate: P, err_message: &'static str) -> Self
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.ensure(
            move |items| items.iter().any(&predicate),
            move |_| ValidationError::InvalidFormat {
                field: "collection_items",
                message: err_message.to_string(),
            },
        )
    }

    /// Validates every element in the collection using `item_pipeline`.
    pub fn each(self, item_pipeline: ValidationPipeline<T>) -> Self {
        self.step(move |items| {
            for item in items {
                item_pipeline.validate(item)?;
            }
            Ok(())
        })
    }

    /// Validates the given slice against all pipeline stages (fail-fast).
    pub fn validate(&self, items: &[T]) -> Result<(), ValidationError> {
        self.pipeline.validate(items)
    }

    /// Validates the given slice against all pipeline stages, collecting all errors.
    pub fn validate_all(&self, items: &[T]) -> Result<(), Vec<ValidationError>> {
        self.pipeline.validate_all(items)
    }
}

impl<T: 'static> crate::traits::Validator<[T]> for CollectionValidationPipeline<T> {
    fn validate(&self, target: &[T]) -> Result<(), ValidationError> {
        self.validate(target)
    }
}

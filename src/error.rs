//! Strongly-Typed Validation Errors for EstateStack Sovereign Platform

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core Validation Error Enum covering Domain Invariants, Business Rules, and Schemas
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Field '{field}' format is invalid: {message}")]
    InvalidFormat {
        field: &'static str,
        message: String,
    },

    #[error("{message}")]
    InvalidId { field: String, message: String },

    #[error("Username '{0}' is invalid: must be 3-30 characters, alphanumeric or underscore, cannot start/end with underscore")]
    InvalidUsername(String),

    #[error("Username '{0}' is a restricted system reserved identifier")]
    ReservedUsername(String),

    #[error("Email address '{0}' does not conform to RFC 5322 specifications")]
    InvalidEmail(String),

    #[error("Phone number '{0}' does not conform to ITU-T E.164 sovereign formatting (+ followed by 7-15 digits)")]
    InvalidPhoneNumber(String),

    #[error("Country code '{0}' is not a valid 2-letter ISO sovereign country code")]
    InvalidCountryCode(String),

    #[error("Currency code '{0}' is not a valid 3-letter ISO-4217 sovereign currency")]
    InvalidCurrency(String),

    #[error("{0}")]
    InvalidPassword(String),

    #[error("Financial amount '{field}' cannot be negative: found {amount_cents} cents")]
    NegativeAmount {
        field: &'static str,
        amount_cents: i64,
    },

    #[error("Insufficient funds: available {available_cents} cents is less than requested {requested_cents} cents")]
    InsufficientBalance {
        available_cents: i64,
        requested_cents: i64,
    },

    #[error("Database schema invariant violation: locked amount ({locked_cents} cents) exceeds total balance ({balance_cents} cents)")]
    LockedExceedsBalance {
        locked_cents: i64,
        balance_cents: i64,
    },

    #[error("Solitary passkey guard: cannot revoke sole remaining passkey '{0}'")]
    SolitaryCredentialViolation(String),

    #[error("Solitary auth provider guard: cannot disconnect sole remaining auth provider '{0}'")]
    SolitaryAuthProviderViolation(String),

    #[error("Session has expired")]
    ExpiredSession,

    #[error("Session has been revoked")]
    RevokedSession,

    #[error("Required field '{0}' is missing or empty")]
    FieldRequired(String),

    #[error(
        "Value for '{field}' is out of bounds: expected between {min} and {max}, got {actual}"
    )]
    OutOfBounds {
        field: &'static str,
        min: String,
        max: String,
        actual: String,
    },

    #[error("Entity schema invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Business validation failure: {0}")]
    Custom(String),
}

impl ValidationError {
    /// Maps to canonical EstateStack error code string
    pub fn error_code(&self) -> &'static str {
        match self {
            ValidationError::InvalidUsername(_) => "ERR_AUTH_001_FORMAT",
            ValidationError::ReservedUsername(_) => "ERR_AUTH_001_RESERVED",
            ValidationError::InvalidEmail(_) => "ERR_AUTH_002_FORMAT",
            ValidationError::InvalidPhoneNumber(_) => "ERR_AUTH_003_FORMAT",
            ValidationError::InvalidCountryCode(_) => "ERR_AUTH_004_FORMAT",
            ValidationError::InvalidPassword(_) => "ERR_AUTH_007_PASSWORD",
            ValidationError::SolitaryCredentialViolation(_) => "ERR_AUTH_010",
            ValidationError::SolitaryAuthProviderViolation(_) => "ERR_AUTH_011",
            ValidationError::ExpiredSession => "ERR_AUTH_005",
            ValidationError::RevokedSession => "ERR_AUTH_006",
            ValidationError::InvalidCurrency(_) => "ERR_WAL_001",
            ValidationError::NegativeAmount { .. } => "ERR_WAL_002",
            ValidationError::InsufficientBalance { .. } => "ERR_WAL_003",
            ValidationError::LockedExceedsBalance { .. } => "ERR_WAL_004",
            ValidationError::FieldRequired(_) => "ERR_VAL_REQUIRED",
            ValidationError::InvalidId { .. } => "ERR_VAL_ID",
            ValidationError::OutOfBounds { .. } => "ERR_VAL_BOUNDS",
            ValidationError::InvalidFormat { .. } => "ERR_VAL_FORMAT",
            ValidationError::InvariantViolation(_) => "ERR_DB_INVARIANT",
            ValidationError::Custom(_) => "ERR_VAL_BUSINESS",
        }
    }
}

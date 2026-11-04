//! Abstract Validation Traits for Invariant & Business Validation

use crate::error::ValidationError;

/// Trait implemented by data structures and database schemas that possess intrinsic self-validation invariants.
pub trait Validate {
    /// Validates internal consistency and invariant rules.
    /// Fails fast on the first invariant violation.
    fn validate(&self) -> Result<(), ValidationError>;
}

/// Trait implemented by structures whose validation requires external context (e.g., system settings, timestamps).
pub trait ValidateWith<Context> {
    /// Validates internal consistency with respect to an external context object.
    fn validate_with(&self, ctx: &Context) -> Result<(), ValidationError>;
}

/// Abstract reusable validator strategy for validating a specific type.
pub trait Validator<Target: ?Sized> {
    fn validate(&self, target: &Target) -> Result<(), ValidationError>;
}

/// Dynamic closure validator helper
impl<Target: ?Sized, F> Validator<Target> for F
where
    F: Fn(&Target) -> Result<(), ValidationError>,
{
    fn validate(&self, target: &Target) -> Result<(), ValidationError> {
        (self)(target)
    }
}

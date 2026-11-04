//! Optional gRPC transport adapters.
//!
//! Validation rules and errors remain framework-neutral. Consumers that use
//! Tonic can enable the `tonic` feature to translate canonical validation
//! failures at their transport boundary.

use crate::ValidationError;

/// Maps a framework-neutral validation result into a Tonic invalid-argument status.
pub fn validation_status<T>(result: Result<T, ValidationError>) -> Result<T, ::tonic::Status> {
    result.map_err(|error| ::tonic::Status::invalid_argument(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_validation_errors_without_changing_success_values() {
        assert_eq!(
            validation_status(Ok::<_, ValidationError>("value")).unwrap(),
            "value"
        );

        let error = validation_status(crate::validate_id("", "user_id")).unwrap_err();
        assert_eq!(error.code(), ::tonic::Code::InvalidArgument);
        assert!(error.message().contains("user_id"));
    }
}

//! # EstateStack Sovereign Abstract Validation Framework
//!
//! Provides abstract, domain-agnostic, zero-I/O validation pipelines, traits, and primitive
//! validators for datatypes, identifiers, and optional transport boundaries.
//!
//! ## Overview
//!
//! The `some-validation` crate provides general-purpose validation primitives and pipelines:
//! - **Pipelines**: [`ValidationPipeline<T>`], [`StringValidationPipeline`], [`NumericValidationPipeline<N>`], [`BytesValidationPipeline`], and [`CollectionValidationPipeline<T>`].
//! - **Primitives**: Usernames, RFC 5322 emails, ITU-T E.164 phone numbers, ISO-4217 currencies, ISO-3166-1 country codes, UUIDs, entity IDs, and configurable password policies ([`PasswordConfig`]).
//! - **Country Codes**: Worldwide ITU-T dialing prefixes, Canadian NANP NPA mappings, and ISO-2 metadata loaded from centralized data.
//! - **Transport Adapters**: Optional framework integration (e.g. `tonic` feature).
//!
//! ## Examples
//!
//! ```rust
//! use some_validation::{validate_id, validate_username, validate_email, validate_phone_number};
//!
//! // Validate entity identifier
//! let user_id = validate_id("usr_01H7XZA8", "user_id").unwrap();
//! assert_eq!(user_id, "usr_01H7XZA8");
//!
//! // Validate username
//! let username = validate_username("kareem_99").unwrap();
//! assert_eq!(username, "kareem_99");
//!
//! // Validate email
//! let email = validate_email("investor@estatestack.io").unwrap();
//! assert_eq!(email, "investor@estatestack.io");
//!
//! // Validate phone number
//! let phone = validate_phone_number("+201001234567").unwrap();
//! assert_eq!(phone, "+201001234567");
//! ```

pub mod error;
pub mod pipeline;
pub mod primitives;
pub mod traits;

#[cfg(feature = "tonic")]
pub mod tonic;

pub use error::ValidationError;
pub use pipeline::*;
pub use primitives::*;
pub use traits::*;

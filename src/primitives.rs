use crate::error::ValidationError;
use crate::pipeline::{
    CollectionValidationPipeline, NumericValidationPipeline, StringValidationPipeline,
};
use crate::traits::{ValidateWith, Validator};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Sovereign Country Dialing & Currency Metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CountryCodeInfo {
    pub name: String,
    pub iso2: String,
    pub iso3: String,
    pub dial_code: String,
    pub currency: String,
    pub flag: String,
}

static COUNTRY_CODES_JSON: &str = include_str!("../data/country_codes.json");
static COUNTRIES_REGISTRY: OnceLock<Vec<CountryCodeInfo>> = OnceLock::new();

/// Returns the complete registry of all worldwide countries and territories
pub fn get_all_country_codes() -> &'static [CountryCodeInfo] {
    COUNTRIES_REGISTRY.get_or_init(|| {
        serde_json::from_str(COUNTRY_CODES_JSON)
            .expect("Corrupt or invalid embedded country_codes.json")
    })
}

/// Reserved system words prohibited from being registered as usernames
pub const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "administrator",
    "root",
    "system",
    "estatestack",
    "support",
    "help",
    "official",
    "compliance",
    "cma",
    "sama",
    "fra",
    "security",
    "mod",
    "moderator",
    "finance",
    "billing",
    "treasury",
    "escrow",
    "custodian",
    "null",
    "undefined",
    "superuser",
    "operator",
];

/// Validates and normalizes a sovereign username.
///
/// # Rules
///
/// - Length must be between 3 and 30 characters inclusive
/// - Lowercase alphanumeric characters (`a-z`, `0-9`) and underscores (`_`) only
/// - Cannot start or end with an underscore
/// - Cannot match any restricted system handles in [`RESERVED_USERNAMES`]
///
/// # Arguments
///
/// * `raw` - The raw username input string to validate and normalize
///
/// # Errors
///
/// Returns [`ValidationError::InvalidUsername`] if format or length criteria are violated,
/// or [`ValidationError::ReservedUsername`] if the username matches a restricted platform handle.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_username;
///
/// let user = validate_username("kareem_99").unwrap();
/// assert_eq!(user, "kareem_99");
///
/// let normalized = validate_username("  Ahmed_Ali  ").unwrap();
/// assert_eq!(normalized, "ahmed_ali");
///
/// assert!(validate_username("admin").is_err());
/// assert!(validate_username("ab").is_err());
/// ```
pub fn validate_username(raw: &str) -> Result<String, ValidationError> {
    let normalized = raw.trim().to_lowercase();

    if let Err(e) = StringValidationPipeline::new()
        .bytes_in_range("username", 3, 30)
        .does_not_start_with("_")
        .does_not_end_with("_")
        .ascii_alphanumeric_or(&['_'])
        .ensure(
            |s| !RESERVED_USERNAMES.contains(&s),
            |s| ValidationError::ReservedUsername(s.to_string()),
        )
        .validate(&normalized)
    {
        return Err(match e {
            ValidationError::ReservedUsername(u) => ValidationError::ReservedUsername(u),
            _ => ValidationError::InvalidUsername(normalized),
        });
    }

    Ok(normalized)
}

/// Validates and normalizes an email address according to RFC 5322 specifications.
///
/// # Rules
///
/// - Total length between 3 and 254 characters inclusive
/// - Exactly one `@` symbol separating local part and domain
/// - Local part <= 64 chars, cannot start/end with a dot or have consecutive dots
/// - Domain part <= 253 chars, valid DNS labels separated by dots
/// - Top-level domain (TLD) must contain at least 2 alphabetic characters
///
/// # Arguments
///
/// * `raw` - The raw email address string to validate and normalize
///
/// # Errors
///
/// Returns [`ValidationError::InvalidEmail`] if the email fails RFC 5322 syntax validation.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_email;
///
/// let email = validate_email("Investor@EstateStack.io").unwrap();
/// assert_eq!(email, "investor@estatestack.io");
///
/// assert!(validate_email("invalid-email").is_err());
/// assert!(validate_email("@missing-local.com").is_err());
/// ```
pub fn validate_email(raw: &str) -> Result<String, ValidationError> {
    let normalized = raw.trim().to_lowercase();

    if StringValidationPipeline::new()
        .bytes_in_range("email", 3, 254)
        .has_part_count('@', 2)
        .step(|s| {
            let parts: Vec<&str> = s.split('@').collect();
            let local = parts[0];
            let domain = parts[1];

            StringValidationPipeline::new()
                .bytes_in_range("email_local", 1, 64)
                .does_not_start_with(".")
                .does_not_end_with(".")
                .does_not_contain("..")
                .validate(local)?;

            StringValidationPipeline::new()
                .bytes_in_range("email_domain", 1, 253)
                .min_part_count('.', 2)
                .validate(domain)?;

            let label_pipeline = StringValidationPipeline::new()
                .bytes_in_range("dns_label", 1, 63)
                .does_not_start_with("-")
                .does_not_end_with("-")
                .ascii_alphanumeric_or(&['-']);

            let domain_parts: Vec<&str> = domain.split('.').collect();
            for part in &domain_parts {
                label_pipeline.validate(part)?;
            }

            let tld = domain_parts.last().unwrap_or(&"");
            StringValidationPipeline::new()
                .min_bytes(2)
                .alphabetic_only()
                .validate(tld)
        })
        .validate(&normalized)
        .is_err()
    {
        return Err(ValidationError::InvalidEmail(normalized));
    }

    Ok(normalized)
}

/// Validates an ITU-T E.164 sovereign telephone number format (+ followed by 7-15 digits).
///
/// # Rules
///
/// - Starts with `+` international prefix
/// - Contains 7 to 15 digits following the prefix
/// - Strips whitespace and hyphens during normalization
/// - Country calling code cannot start with `0`
///
/// # Arguments
///
/// * `raw` - The raw telephone number string to validate
///
/// # Errors
///
/// Returns [`ValidationError::InvalidPhoneNumber`] if formatting violates ITU-T E.164.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_phone_number;
///
/// let phone = validate_phone_number("+20 100-123-4567").unwrap();
/// assert_eq!(phone, "+201001234567");
///
/// assert!(validate_phone_number("01001234567").is_err());
/// assert!(validate_phone_number("+20").is_err());
/// ```
pub fn validate_phone_number(raw: &str) -> Result<String, ValidationError> {
    let stripped: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();

    if StringValidationPipeline::new()
        .starts_with("+")
        .step(|s| {
            let digits = &s[1..];
            StringValidationPipeline::new()
                .bytes_in_range("phone_digits", 7, 15)
                .numeric_only()
                .does_not_start_with("0")
                .validate(digits)
        })
        .validate(&stripped)
        .is_err()
    {
        return Err(ValidationError::InvalidPhoneNumber(stripped));
    }

    Ok(stripped)
}

/// Validates an ISO-4217 3-letter currency code (e.g. "USD", "SAR", "EGP", "AED").
///
/// Converts the currency code to uppercase and ensures it consists of exactly 3 alphabetic characters.
///
/// # Arguments
///
/// * `raw` - The raw 3-letter currency string to validate
///
/// # Errors
///
/// Returns [`ValidationError::InvalidCurrency`] if code length is not 3 or contains non-alphabetic characters.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_currency_code;
///
/// assert_eq!(validate_currency_code("usd").unwrap(), "USD");
/// assert_eq!(validate_currency_code("SAR").unwrap(), "SAR");
/// assert!(validate_currency_code("US").is_err());
/// assert!(validate_currency_code("123").is_err());
/// ```
pub fn validate_currency_code(raw: &str) -> Result<String, ValidationError> {
    let normalized = raw.trim().to_uppercase();
    if StringValidationPipeline::new()
        .exact_bytes(3)
        .alphabetic_only()
        .validate(&normalized)
        .is_err()
    {
        return Err(ValidationError::InvalidCurrency(normalized));
    }
    Ok(normalized)
}

/// Validates that a string is a valid RFC 4122 UUID using a [`StringValidationPipeline`].
///
/// # Arguments
///
/// * `raw` - The string representation of a UUID to validate
///
/// # Errors
///
/// Returns [`ValidationError::InvalidFormat`] if the input string fails standard UUID parsing.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_uuid;
///
/// let parsed = validate_uuid("550e8400-e29b-41d4-a716-446655440000").unwrap();
/// assert_eq!(parsed.to_string(), "550e8400-e29b-41d4-a716-446655440000");
/// assert!(validate_uuid("invalid-uuid").is_err());
/// ```
pub fn validate_uuid(raw: &str) -> Result<uuid::Uuid, ValidationError> {
    StringValidationPipeline::new()
        .non_empty("uuid")
        .uuid()
        .validate(raw)?;
    uuid::Uuid::parse_str(raw.trim()).map_err(|_| ValidationError::InvalidFormat {
        field: "uuid",
        message: format!("'{}' is not a valid UUID", raw),
    })
}

/// Validates a prefixed UUID identifier (e.g., `usr_<uuid>`, `cch_<uuid>`) via [`StringValidationPipeline`].
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_prefixed_uuid;
///
/// let valid = validate_prefixed_uuid("usr_", "usr_550e8400-e29b-41d4-a716-446655440000").unwrap();
/// assert_eq!(valid, "usr_550e8400-e29b-41d4-a716-446655440000");
/// assert!(validate_prefixed_uuid("usr_", "cch_550e8400-e29b-41d4-a716-446655440000").is_err());
/// ```
pub fn validate_prefixed_uuid<'a>(
    prefix: &'static str,
    raw: &'a str,
) -> Result<&'a str, ValidationError> {
    let trimmed = raw.trim();
    StringValidationPipeline::new()
        .non_empty("prefixed_uuid")
        .prefixed_uuid(prefix)
        .validate(trimmed)?;
    Ok(trimmed)
}

/// Abstract configuration for validating MongoDB document `_id` strings or 24-hex ObjectIds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MongodbIdConfig {
    /// Whether the ID must strictly be a 24-character hexadecimal BSON ObjectId.
    /// When `false`, allows custom string document `_id` values up to `max_bytes` with safe characters.
    pub strict_object_id: bool,
    /// Minimum byte length when `strict_object_id` is `false`. Default: 1.
    pub min_bytes: usize,
    /// Maximum byte length when `strict_object_id` is `false`. Default: 128.
    pub max_bytes: usize,
    /// Optional required prefix (e.g. `Some("usr_")`).
    pub required_prefix: Option<String>,
}

impl Default for MongodbIdConfig {
    fn default() -> Self {
        Self {
            strict_object_id: true,
            min_bytes: 1,
            max_bytes: 128,
            required_prefix: None,
        }
    }
}

impl MongodbIdConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_bytes(mut self, min_bytes: usize) -> Self {
        self.min_bytes = min_bytes;
        self
    }

    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn with_strict_object_id(mut self, strict: bool) -> Self {
        self.strict_object_id = strict;
        self
    }
    /// Creates a strict 24-character hex MongoDB ObjectId validation config.
    pub fn strict_object_id() -> Self {
        Self::default()
    }

    /// Creates a general MongoDB document `_id` string config (allowing alphanumeric, `_`, `-`, `:`).
    pub fn document_id() -> Self {
        Self {
            strict_object_id: false,
            min_bytes: 1,
            max_bytes: 128,
            required_prefix: None,
        }
    }

    /// Sets an optional required prefix for the MongoDB document `_id`.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.required_prefix = Some(prefix.into());
        self
    }

    /// Builds a [`StringValidationPipeline`] for this MongoDB ID policy.
    pub fn pipeline(&self) -> StringValidationPipeline {
        if self.strict_object_id {
            StringValidationPipeline::new()
                .non_empty("mongodb_id")
                .mongodb_object_id()
        } else {
            let mut p = StringValidationPipeline::new()
                .non_empty("mongodb_id")
                .bytes_in_range("mongodb_id", self.min_bytes, self.max_bytes)
                .no_whitespace()
                .no_control_chars()
                .ascii_alphanumeric_or(&['_', '-', ':']);
            if let Some(ref prefix) = self.required_prefix {
                p = p.starts_with(prefix.as_str());
            }
            p
        }
    }

    /// Validates a MongoDB identifier string against this configuration.
    pub fn validate<'a>(&self, raw: &'a str) -> Result<&'a str, ValidationError> {
        let trimmed = raw.trim();
        self.pipeline().validate(trimmed)?;
        Ok(trimmed)
    }
}

impl Validator<str> for MongodbIdConfig {
    fn validate(&self, target: &str) -> Result<(), ValidationError> {
        self.pipeline().validate(target)
    }
}

impl ValidateWith<MongodbIdConfig> for str {
    fn validate_with(&self, config: &MongodbIdConfig) -> Result<(), ValidationError> {
        config.pipeline().validate(self)
    }
}

/// Validates a 24-character hexadecimal MongoDB ObjectId string using [`StringValidationPipeline`].
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_mongodb_id;
///
/// assert!(validate_mongodb_id("507f1f77bcf86cd799439011").is_ok());
/// assert!(validate_mongodb_id("not-an-object-id").is_err());
/// ```
pub fn validate_mongodb_id(raw: &str) -> Result<&str, ValidationError> {
    MongodbIdConfig::strict_object_id().validate(raw)
}

/// Validates a MongoDB document `_id` against a custom [`MongodbIdConfig`].
pub fn validate_mongodb_id_with_config<'a>(
    config: &MongodbIdConfig,
    raw: &'a str,
) -> Result<&'a str, ValidationError> {
    config.validate(raw)
}

/// Validates an N-digit numeric verification code (e.g., 6-digit OTP or TOTP code) via [`StringValidationPipeline`].
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_numeric_code;
///
/// assert_eq!(validate_numeric_code("otp", 6, " 123456 ").unwrap(), "123456");
/// assert!(validate_numeric_code("otp", 6, "12345a").is_err());
/// ```
pub fn validate_numeric_code<'a>(
    field: &'static str,
    digits: usize,
    raw: &'a str,
) -> Result<&'a str, ValidationError> {
    let trimmed = raw.trim();
    StringValidationPipeline::new()
        .non_empty(field)
        .exact_length(digits)
        .numeric_only()
        .map_error(move |_| ValidationError::InvalidFormat {
            field,
            message: format!("'{}' must be exactly {} numeric digits", field, digits),
        })
        .validate(trimmed)?;
    Ok(trimmed)
}

/// Validates a hexadecimal digest string of `expected_hex_len` characters (e.g., 64 for SHA-256)
/// via [`StringValidationPipeline`].
pub fn validate_hex_digest<'a>(
    field: &'static str,
    expected_hex_len: usize,
    raw: &'a str,
) -> Result<&'a str, ValidationError> {
    let trimmed = raw.trim();
    StringValidationPipeline::new()
        .non_empty(field)
        .exact_bytes(expected_hex_len)
        .hex_only()
        .map_error(move |_| ValidationError::InvalidFormat {
            field,
            message: format!(
                "'{}' must be a {}-character hexadecimal digest",
                field, expected_hex_len
            ),
        })
        .validate(trimmed)?;
    Ok(trimmed)
}

/// Validates a two-part composite token (`<first><delimiter><second>`) via [`StringValidationPipeline`]
/// and returns borrowed `(first, second)` slices without allocating or cloning.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_composite_token;
///
/// let (cid, tok) = validate_composite_token("intermediate_2fa_token", ':', "cch_123:tok_456").unwrap();
/// assert_eq!(cid, "cch_123");
/// assert_eq!(tok, "tok_456");
/// assert!(validate_composite_token("intermediate_2fa_token", ':', "malformed").is_err());
/// ```
pub fn validate_composite_token<'a>(
    field: &'static str,
    delimiter: char,
    raw: &'a str,
) -> Result<(&'a str, &'a str), ValidationError> {
    let trimmed = raw.trim();
    StringValidationPipeline::new()
        .non_empty(field)
        .has_part_count(delimiter, 2)
        .validate(trimmed)
        .map_err(|_| ValidationError::InvalidFormat {
            field,
            message: format!("Malformed composite token for '{}'", field),
        })?;
    let (first, second) =
        trimmed
            .split_once(delimiter)
            .ok_or_else(|| ValidationError::InvalidFormat {
                field,
                message: format!("Malformed composite token for '{}'", field),
            })?;
    let first = first.trim();
    let second = second.trim();
    StringValidationPipeline::new()
        .non_empty(field)
        .validate(first)?;
    StringValidationPipeline::new()
        .non_empty(field)
        .validate(second)?;
    Ok((first, second))
}

/// Validates that a required string field is non-empty after trimming leading and trailing whitespace.
///
/// # Arguments
///
/// * `field` - The field name for descriptive error reporting
/// * `val` - The string value to inspect
///
/// # Errors
///
/// Returns [`ValidationError::FieldRequired`] if the trimmed string is empty.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_non_empty;
///
/// assert!(validate_non_empty("title", "EstateStack Sovereign").is_ok());
/// assert!(validate_non_empty("title", "   ").is_err());
/// assert!(validate_non_empty("title", "").is_err());
/// ```
pub fn validate_non_empty(field: &str, val: &str) -> Result<(), ValidationError> {
    if StringValidationPipeline::new()
        .ensure(
            |s| !s.trim().is_empty(),
            |_| ValidationError::FieldRequired(String::new()),
        )
        .validate(val)
        .is_err()
    {
        return Err(ValidationError::FieldRequired(field.to_string()));
    }
    Ok(())
}

/// Validates the UTF-8 byte length of a string against inclusive bounds.
pub fn validate_byte_length(
    field: &'static str,
    value: &str,
    min: usize,
    max: usize,
) -> Result<(), ValidationError> {
    StringValidationPipeline::new()
        .bytes_in_range(field, min, max)
        .validate(value)
}

/// Requires a collection to contain at least one value when `required` is true.
pub fn validate_required_collection<T: 'static>(
    field: &str,
    values: &[T],
    required: bool,
) -> Result<(), ValidationError> {
    if CollectionValidationPipeline::<T>::new()
        .ensure_if(
            required,
            |items| !items.is_empty(),
            |_| ValidationError::FieldRequired(String::new()),
        )
        .validate(values)
        .is_err()
    {
        return Err(ValidationError::FieldRequired(field.to_string()));
    }
    Ok(())
}

/// Validates an optional positive integer range when both endpoints are present.
pub fn validate_optional_ordered_i64(
    field: &'static str,
    start: i64,
    end: i64,
) -> Result<(), ValidationError> {
    NumericValidationPipeline::<i64>::new()
        .ensure_if(
            start > 0 && end > 0,
            move |&e| e > start,
            move |&e| ValidationError::InvalidFormat {
                field,
                message: format!("end ({e}) must be greater than start ({start})"),
            },
        )
        .validate(&end)
}

/// Validates that an integer-backed enum discriminant is specified.
pub fn validate_nonzero_i32(field: &'static str, value: i32) -> Result<(), ValidationError> {
    NumericValidationPipeline::<i32>::new()
        .not_equals(field, 0)
        .map_error(move |_| ValidationError::InvalidFormat {
            field,
            message: "unspecified value is not allowed".to_string(),
        })
        .validate(&value)
}

/// Validates that an entity identifier is non-empty and within bounds (1 to 128 chars).
///
/// Trims surrounding whitespace and ensures the identifier meets sovereign platform rules:
/// - Cannot be empty or whitespace-only
/// - Must not exceed 128 characters in length
///
/// Returns the normalized (trimmed) identifier string on success.
///
/// # Arguments
///
/// * `id` - The raw identifier string to validate (e.g. UUID, ULID, database key, or slug)
/// * `field_name` - The descriptive name of the ID field (e.g. `"user_id"`, `"property_id"`)
///
/// # Errors
///
/// Returns [`ValidationError::InvalidId`] if the identifier is empty, whitespace-only,
/// or exceeds 128 characters.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_id;
///
/// let valid = validate_id("usr_01H7XZA8", "user_id").unwrap();
/// assert_eq!(valid, "usr_01H7XZA8");
///
/// let trimmed = validate_id("  property_99  ", "property_id").unwrap();
/// assert_eq!(trimmed, "property_99");
///
/// let err = validate_id("", "wallet_id");
/// assert!(err.is_err());
/// ```
pub fn validate_id(id: &str, field_name: &str) -> Result<String, ValidationError> {
    let trimmed = id.trim();
    StringValidationPipeline::new()
        .ensure(
            |s| !s.is_empty(),
            |_| ValidationError::FieldRequired(String::new()),
        )
        .validate(trimmed)
        .map_err(|_| ValidationError::InvalidId {
            field: field_name.to_string(),
            message: format!("Field '{}' cannot be empty", field_name),
        })?;

    StringValidationPipeline::new()
        .max_bytes(128)
        .validate(trimmed)
        .map_err(|_| ValidationError::InvalidId {
            field: field_name.to_string(),
            message: format!(
                "Field '{}' exceeds maximum identifier length of 128 characters",
                field_name
            ),
        })?;

    Ok(trimmed.to_string())
}

/// Detects the matching country metadata record for a given phone number.
///
/// Resolves Canadian NANP area codes under `+1`, and matches longest dial code prefix first.
///
/// # Arguments
///
/// * `phone` - An E.164 phone number string
///
/// # Examples
///
/// ```rust
/// use some_validation::detect_country_info;
///
/// let info = detect_country_info("+201001234567").unwrap();
/// assert_eq!(info.iso2, "EG");
/// assert_eq!(info.currency, "EGP");
/// ```
pub fn detect_country_info(phone: &str) -> Option<&'static CountryCodeInfo> {
    let clean: String = phone
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if !clean.starts_with('+') {
        return None;
    }

    // Handle Canadian NANP area codes under +1
    if clean.starts_with("+1") && clean.len() >= 5 {
        let npa = &clean[2..5];
        let canadian_npas = [
            "403", "587", "780", "825", "236", "250", "604", "778", "204", "431", "506", "709",
            "902", "782", "226", "249", "289", "343", "365", "416", "437", "519", "548", "613",
            "647", "705", "807", "905", "418", "438", "450", "514", "579", "581", "819", "873",
            "306", "639", "867",
        ];
        if canadian_npas.contains(&npa) {
            return get_all_country_codes().iter().find(|c| c.iso2 == "CA");
        }
    }

    // Match longest dial code prefix first (e.g. +1242 before +1)
    let countries = get_all_country_codes();
    countries
        .iter()
        .filter(|c| clean.starts_with(&c.dial_code))
        .max_by_key(|c| c.dial_code.len())
}

/// Detects ISO country code and default national currency based on ITU-T E.164 phone prefix.
///
/// # Arguments
///
/// * `phone` - An E.164 phone number string
///
/// # Examples
///
/// ```rust
/// use some_validation::detect_country_code;
///
/// assert_eq!(detect_country_code("+201001234567"), Some(("EG", "EGP")));
/// assert_eq!(detect_country_code("+966501234567"), Some(("SA", "SAR")));
/// ```
pub fn detect_country_code(phone: &str) -> Option<(&'static str, &'static str)> {
    detect_country_info(phone).map(|c| (c.iso2.as_str(), c.currency.as_str()))
}

/// Finds country info by ISO-2 code (e.g. "EG", "SA", "US", "GB").
///
/// # Arguments
///
/// * `iso2` - 2-letter ISO country code string (case-insensitive)
///
/// # Examples
///
/// ```rust
/// use some_validation::find_country_by_iso2;
///
/// let egypt = find_country_by_iso2("EG").unwrap();
/// assert_eq!(egypt.name, "Egypt");
/// assert_eq!(egypt.currency, "EGP");
/// ```
pub fn find_country_by_iso2(iso2: &str) -> Option<&'static CountryCodeInfo> {
    let norm = iso2.trim().to_uppercase();
    get_all_country_codes().iter().find(|c| c.iso2 == norm)
}

/// Validates that a country code is a valid ISO-3166-1 alpha-2 sovereign code.
///
/// Validates against the embedded 229 worldwide sovereign countries and territories.
///
/// # Arguments
///
/// * `raw` - 2-letter ISO country code string (case-insensitive)
///
/// # Errors
///
/// Returns [`ValidationError::InvalidCountryCode`] if code is not recognized.
///
/// # Examples
///
/// ```rust
/// use some_validation::validate_country_code;
///
/// assert_eq!(validate_country_code("eg").unwrap(), "EG");
/// assert_eq!(validate_country_code("SA").unwrap(), "SA");
/// assert!(validate_country_code("ZZ").is_err());
/// ```
pub fn validate_country_code(raw: &str) -> Result<String, ValidationError> {
    let norm = raw.trim().to_uppercase();
    if find_country_by_iso2(&norm).is_some() {
        Ok(norm)
    } else {
        Err(ValidationError::InvalidCountryCode(norm))
    }
}

/// Resolves ISO-2 country code from phone number and optional user-provided ISO-2 country hint.
///
/// If hint is provided and valid, it takes precedence.
/// Otherwise detects from E.164 prefix, falling back to "US" if unrecognized.
///
/// # Arguments
///
/// * `phone` - E.164 phone number string
/// * `country_hint` - Optional user-provided ISO-2 country hint
///
/// # Examples
///
/// ```rust
/// use some_validation::resolve_country_code;
///
/// assert_eq!(resolve_country_code("+201001234567", None), "EG");
/// assert_eq!(resolve_country_code("+201001234567", Some("ae")), "AE");
/// assert_eq!(resolve_country_code("+88812345678", None), "US");
/// ```
pub fn resolve_country_code(phone: &str, country_hint: Option<&str>) -> String {
    if let Some(hint) = country_hint {
        let h = hint.trim().to_uppercase();
        if !h.is_empty() && find_country_by_iso2(&h).is_some() {
            return h;
        }
    }
    detect_country_code(phone)
        .map(|(iso, _)| iso.to_string())
        .unwrap_or_else(|| "US".to_string())
}

// ============================================================================
// Configurable Password Validation Policy & Pipeline
// ============================================================================

/// Default list of commonly compromised passwords rejected offline.
pub const DEFAULT_COMPROMISED_PASSWORDS: &[&str] = &[
    "password123!",
    "password1234!",
    "admin12345!",
    "welcome123!",
    "secret12345!",
    "trustnoone1!",
    "qwerty12345!",
    "letmein12345!",
    "iloveyou123!",
    "changeme123!",
];

/// Abstract and configurable password validation policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PasswordConfig {
    /// Minimum password length in Unicode characters (scalar values). Default: 10.
    pub min_length: usize,
    /// Maximum password length in bytes to prevent DoS attacks. Default: 128.
    pub max_bytes: usize,
    /// Whether at least one uppercase ASCII letter is required. Default: true.
    pub require_uppercase: bool,
    /// Whether at least one lowercase ASCII letter is required. Default: true.
    pub require_lowercase: bool,
    /// Whether at least one numeric digit is required. Default: true.
    pub require_digit: bool,
    /// Whether at least one special symbol / punctuation character is required. Default: true.
    pub require_symbol: bool,
    /// Whether ASCII control characters are prohibited. Default: true.
    pub disallow_control_chars: bool,
    /// Prohibited / compromised passwords (checked case-insensitively).
    pub disallowed_passwords: Vec<String>,
}

/// Type alias for [`PasswordConfig`].
pub type PasswordValidationConfig = PasswordConfig;

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 10,
            max_bytes: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_symbol: true,
            disallow_control_chars: true,
            disallowed_passwords: Vec::new(),
        }
    }
}

impl PasswordConfig {
    /// Creates a new default password policy configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum length in Unicode characters.
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = min_length;
        self
    }

    /// Sets the maximum length in bytes.
    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// Configures uppercase requirement.
    pub fn with_require_uppercase(mut self, require: bool) -> Self {
        self.require_uppercase = require;
        self
    }

    /// Configures lowercase requirement.
    pub fn with_require_lowercase(mut self, require: bool) -> Self {
        self.require_lowercase = require;
        self
    }

    /// Configures digit requirement.
    pub fn with_require_digit(mut self, require: bool) -> Self {
        self.require_digit = require;
        self
    }

    /// Configures special symbol requirement.
    pub fn with_require_symbol(mut self, require: bool) -> Self {
        self.require_symbol = require;
        self
    }

    /// Configures control character prohibition.
    pub fn with_disallow_control_chars(mut self, disallow: bool) -> Self {
        self.disallow_control_chars = disallow;
        self
    }

    /// Configures disallowed passwords list.
    pub fn with_disallowed_passwords<I, S>(mut self, passwords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disallowed_passwords = passwords.into_iter().map(Into::into).collect();
        self
    }

    /// Populates the disallowed list with the standard compromised passwords.
    pub fn with_default_compromised_passwords(mut self) -> Self {
        self.disallowed_passwords = DEFAULT_COMPROMISED_PASSWORDS
            .iter()
            .map(|&s| s.to_string())
            .collect();
        self
    }

    /// Builds an abstract [`StringValidationPipeline`] encapsulating this password policy configuration.
    pub fn pipeline(&self) -> StringValidationPipeline {
        let min = self.min_length;
        let max = self.max_bytes;
        let disallowed: Vec<_> = self
            .disallowed_passwords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        StringValidationPipeline::new()
            .ensure_if(
                min > 0,
                move |s| s.chars().count() >= min,
                move |_| {
                    ValidationError::InvalidPassword(format!(
                        "Password must be at least {} characters long",
                        min
                    ))
                },
            )
            .ensure_if(
                max > 0,
                move |s| s.len() <= max,
                move |_| {
                    ValidationError::InvalidPassword(format!(
                        "Password exceeds maximum allowed length of {} bytes",
                        max
                    ))
                },
            )
            .ensure_if(
                self.disallow_control_chars,
                |s| !s.chars().any(|c| c.is_control()),
                |_| {
                    ValidationError::InvalidPassword(
                        "Password must not contain control characters".to_string(),
                    )
                },
            )
            .ensure_if(
                self.require_uppercase,
                |s| s.chars().any(|c| c.is_ascii_uppercase()),
                |_| {
                    ValidationError::InvalidPassword(
                        "Password must contain at least one uppercase letter".to_string(),
                    )
                },
            )
            .ensure_if(
                self.require_lowercase,
                |s| s.chars().any(|c| c.is_ascii_lowercase()),
                |_| {
                    ValidationError::InvalidPassword(
                        "Password must contain at least one lowercase letter".to_string(),
                    )
                },
            )
            .ensure_if(
                self.require_digit,
                |s| s.chars().any(|c| c.is_ascii_digit()),
                |_| {
                    ValidationError::InvalidPassword(
                        "Password must contain at least one digit".to_string(),
                    )
                },
            )
            .ensure_if(
                self.require_symbol,
                |s| {
                    s.chars().any(|c| {
                        c.is_ascii_punctuation()
                            || (!c.is_alphanumeric() && !c.is_whitespace() && !c.is_control())
                    })
                },
                |_| {
                    ValidationError::InvalidPassword(
                        "Password must contain at least one special symbol".to_string(),
                    )
                },
            )
            .ensure_if(
                !self.disallowed_passwords.is_empty(),
                move |s| {
                    let lower = s.to_lowercase();
                    !disallowed.iter().any(|d| d == &lower)
                },
                |_| {
                    ValidationError::InvalidPassword(
                        "Password is too common and has been compromised in known breaches"
                            .to_string(),
                    )
                },
            )
    }

    /// Validates a password against this configuration using the underlying validation pipeline.
    pub fn validate(&self, password: &str) -> Result<(), ValidationError> {
        self.pipeline().validate(password)
    }
}

/// Validates a candidate password string against the provided abstract [`PasswordConfig`]
/// using an underlying [`StringValidationPipeline`].
pub fn validate_password(config: &PasswordConfig, password: &str) -> Result<(), ValidationError> {
    config.pipeline().validate(password)
}

impl crate::traits::Validator<str> for PasswordConfig {
    fn validate(&self, target: &str) -> Result<(), ValidationError> {
        validate_password(self, target)
    }
}

impl crate::traits::ValidateWith<PasswordConfig> for str {
    fn validate_with(&self, ctx: &PasswordConfig) -> Result<(), ValidationError> {
        validate_password(ctx, self)
    }
}

impl crate::traits::ValidateWith<PasswordConfig> for String {
    fn validate_with(&self, ctx: &PasswordConfig) -> Result<(), ValidationError> {
        validate_password(ctx, self.as_str())
    }
}

/// Validates that a financial amount in cents is non-negative (>= 0).
pub fn validate_non_negative_amount(field: &'static str, amount_cents: i64) -> Result<(), ValidationError> {
    if amount_cents < 0 {
        return Err(ValidationError::NegativeAmount {
            field,
            amount_cents,
        });
    }
    Ok(())
}

/// Validates that a financial amount in cents is strictly positive (> 0).
pub fn validate_positive_amount(field: &'static str, amount_cents: i64) -> Result<(), ValidationError> {
    if amount_cents <= 0 {
        return Err(ValidationError::OutOfBounds {
            field,
            min: "1 cent".to_string(),
            max: "unlimited".to_string(),
            actual: format!("{} cents", amount_cents),
        });
    }
    Ok(())
}

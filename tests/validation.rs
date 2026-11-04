use some_validation::error::ValidationError;
use some_validation::primitives::*;

#[test]
fn test_validate_username_success() {
    assert_eq!(validate_username("kareem").unwrap(), "kareem");
    assert_eq!(validate_username("  Kareem_99  ").unwrap(), "kareem_99");
    assert_eq!(validate_username("investor_1").unwrap(), "investor_1");
}

#[test]
fn test_validate_username_failures() {
    assert!(matches!(
        validate_username("ab"),
        Err(ValidationError::InvalidUsername(_))
    ));
    assert!(matches!(
        validate_username("a".repeat(31).as_str()),
        Err(ValidationError::InvalidUsername(_))
    ));
    assert!(matches!(
        validate_username("_kareem"),
        Err(ValidationError::InvalidUsername(_))
    ));
    assert!(matches!(
        validate_username("kareem_"),
        Err(ValidationError::InvalidUsername(_))
    ));
    assert!(matches!(
        validate_username("admin"),
        Err(ValidationError::ReservedUsername(_))
    ));
    assert!(matches!(
        validate_username("ROOT"),
        Err(ValidationError::ReservedUsername(_))
    ));
}

#[test]
fn test_validate_email() {
    assert_eq!(
        validate_email("user@estatestack.io").unwrap(),
        "user@estatestack.io"
    );
    assert_eq!(
        validate_email("  User.Name@Domain.CO.UK  ").unwrap(),
        "user.name@domain.co.uk"
    );
    assert!(matches!(
        validate_email("invalid"),
        Err(ValidationError::InvalidEmail(_))
    ));
    assert!(matches!(
        validate_email("@domain.com"),
        Err(ValidationError::InvalidEmail(_))
    ));
    assert!(matches!(
        validate_email("user@"),
        Err(ValidationError::InvalidEmail(_))
    ));
    assert!(matches!(
        validate_email("user@domain"),
        Err(ValidationError::InvalidEmail(_))
    ));
}

#[test]
fn test_validate_phone_and_country_detection() {
    assert_eq!(
        validate_phone_number("+20 100-123-4567").unwrap(),
        "+201001234567"
    );
    assert_eq!(detect_country_code("+201001234567"), Some(("EG", "EGP")));
    assert_eq!(detect_country_code("+966501234567"), Some(("SA", "SAR")));
    assert_eq!(detect_country_code("+971501234567"), Some(("AE", "AED")));
    assert_eq!(detect_country_code("+14161234567"), Some(("CA", "CAD")));
    assert_eq!(detect_country_code("+12125551234"), Some(("US", "USD")));
    assert!(matches!(
        validate_phone_number("01001234567"),
        Err(ValidationError::InvalidPhoneNumber(_))
    ));

    // Country validation
    assert_eq!(validate_country_code("eg").unwrap(), "EG");
    assert_eq!(validate_country_code("SA").unwrap(), "SA");
    assert!(matches!(
        validate_country_code("XX"),
        Err(ValidationError::InvalidCountryCode(_))
    ));

    // Country resolution with hint & fallback
    assert_eq!(resolve_country_code("+201001234567", None), "EG");
    assert_eq!(resolve_country_code("+201001234567", Some("ae")), "AE");
    assert_eq!(resolve_country_code("+88812345678", None), "US"); // Fallback
}

#[test]
fn test_validate_id() {
    assert_eq!(validate_id("user_123", "user_id").unwrap(), "user_123");
    assert_eq!(
        validate_id("  property_456  ", "property_id").unwrap(),
        "property_456"
    );
    assert_eq!(
        validate_id("valid-uuid-123", "test_id").unwrap(),
        "valid-uuid-123"
    );

    // Empty ID
    let err_empty = validate_id("", "user_id").unwrap_err();
    assert_eq!(err_empty.to_string(), "Field 'user_id' cannot be empty");
    assert_eq!(err_empty.error_code(), "ERR_VAL_ID");

    // Whitespace only
    let err_ws = validate_id("   \t  ", "account_id").unwrap_err();
    assert_eq!(err_ws.to_string(), "Field 'account_id' cannot be empty");

    // Exceeds max length
    let err_len = validate_id(&"x".repeat(129), "long_id").unwrap_err();
    assert!(err_len
        .to_string()
        .contains("exceeds maximum identifier length"));
}

#[test]
fn test_generic_length_uuid_and_collection_validation() {
    assert!(validate_byte_length("audit_reason", "review", 1, 500).is_ok());
    assert!(validate_byte_length("audit_reason", "", 1, 500).is_err());
    assert!(validate_byte_length("audit_reason", &"x".repeat(501), 1, 500).is_err());

    let id = uuid::Uuid::new_v4();
    assert_eq!(validate_uuid(&id.to_string()), Ok(id));
    assert!(validate_uuid("not-a-uuid").is_err());

    assert!(validate_required_collection("ids", &["one"], true).is_ok());
    assert!(validate_required_collection::<String>("ids", &[], false).is_ok());
    assert!(validate_required_collection::<String>("ids", &[], true).is_err());
}

#[test]
fn test_generic_range_and_discriminant_validation() {
    assert!(validate_optional_ordered_i64("schedule", 100, 200).is_ok());
    assert!(validate_optional_ordered_i64("schedule", 0, 0).is_ok());
    assert!(validate_optional_ordered_i64("schedule", 200, 100).is_err());
    assert!(validate_nonzero_i32("interaction_type", 1).is_ok());
    assert!(validate_nonzero_i32("interaction_type", 0).is_err());
}

#[cfg(test)]
mod ng_tests {
    use super::*;
    use some_validation::error::ValidationError;

    #[test]
    fn test_ng_validate_username_invalid_chars_and_reserved() {
        assert!(matches!(
            validate_username(""),
            Err(ValidationError::InvalidUsername(_))
        ));
        assert!(matches!(
            validate_username("  "),
            Err(ValidationError::InvalidUsername(_))
        ));
        assert!(matches!(
            validate_username("user!name"),
            Err(ValidationError::InvalidUsername(_))
        ));
        assert!(matches!(
            validate_username("user@domain"),
            Err(ValidationError::InvalidUsername(_))
        ));
        assert!(matches!(
            validate_username("root"),
            Err(ValidationError::ReservedUsername(_))
        ));
        assert!(matches!(
            validate_username("ADMINISTRATOR"),
            Err(ValidationError::ReservedUsername(_))
        ));
    }

    #[test]
    fn test_ng_validate_email_malformed_syntax() {
        assert!(matches!(
            validate_email(""),
            Err(ValidationError::InvalidEmail(_))
        ));
        assert!(matches!(
            validate_email("plainaddress"),
            Err(ValidationError::InvalidEmail(_))
        ));
        assert!(matches!(
            validate_email("#@%^%#$@#$@#.com"),
            Err(ValidationError::InvalidEmail(_))
        ));
        assert!(matches!(
            validate_email("@missingusername.com"),
            Err(ValidationError::InvalidEmail(_))
        ));
        assert!(matches!(
            validate_email("username@.com"),
            Err(ValidationError::InvalidEmail(_))
        ));
    }

    #[test]
    fn test_ng_validate_phone_and_country_boundaries() {
        assert!(matches!(
            validate_phone_number(""),
            Err(ValidationError::InvalidPhoneNumber(_))
        ));
        assert!(matches!(
            validate_phone_number("+1"),
            Err(ValidationError::InvalidPhoneNumber(_))
        ));
        assert!(matches!(
            validate_phone_number("+99999999999999999"),
            Err(ValidationError::InvalidPhoneNumber(_))
        ));
        assert!(matches!(
            validate_country_code("ZZZ"),
            Err(ValidationError::InvalidCountryCode(_))
        ));
        assert!(matches!(
            validate_country_code(""),
            Err(ValidationError::InvalidCountryCode(_))
        ));
    }

    #[test]
    fn test_validate_password_abstract_config() {
        use some_validation::traits::{ValidateWith, Validator};

        let default_config = PasswordConfig::default().with_default_compromised_passwords();

        // Valid passwords
        assert!(validate_password(&default_config, "Valid1234_Safe!").is_ok());
        assert!(validate_password(&default_config, "Estate$tack!99").is_ok());
        assert!(validate_password(&default_config, "Kareem#2026").is_ok());

        // Inherent and trait usage: Validator<str>
        assert!(default_config.validate("Valid1234_Safe!").is_ok());
        assert!(Validator::validate(&default_config, "Valid1234_Safe!").is_ok());
        // Trait usage: ValidateWith<PasswordConfig>
        assert!("Valid1234_Safe!".validate_with(&default_config).is_ok());
        assert!("Valid1234_Safe!"
            .to_string()
            .validate_with(&default_config)
            .is_ok());

        // Too short (< 10 chars)
        let err_short = validate_password(&default_config, "Short1!").unwrap_err();
        assert_eq!(err_short.error_code(), "ERR_AUTH_007_PASSWORD");
        assert!(err_short.to_string().contains("at least 10"));

        // Unicode multi-byte length check (< 10 Unicode chars even if > 10 bytes)
        assert!(validate_password(&default_config, "A1!أبجدية").is_err());

        // No uppercase
        assert!(validate_password(&default_config, "lowercase123!").is_err());
        // No lowercase
        assert!(validate_password(&default_config, "UPPERCASE123!").is_err());
        // No digit
        assert!(validate_password(&default_config, "NoDigitsHere!").is_err());
        // No symbol
        assert!(validate_password(&default_config, "NoSymbol12345").is_err());
        // Whitespace as symbol rejected
        assert!(validate_password(&default_config, "Abcdefgh1 ").is_err());
        // Control characters rejected
        assert!(validate_password(&default_config, "Abcdefgh1!\n").is_err());

        // Max length in bytes
        let too_long = "A1!a".repeat(33); // 132 bytes
        assert!(validate_password(&default_config, &too_long).is_err());

        // Compromised / blacklisted
        assert!(validate_password(&default_config, "Password123!").is_err());
        assert!(validate_password(&default_config, "admin12345!").is_err());

        // Custom relaxed configuration
        let relaxed_config = PasswordConfig::new()
            .with_min_length(6)
            .with_require_uppercase(false)
            .with_require_symbol(false);
        assert!(validate_password(&relaxed_config, "simple1").is_ok());
    }

    #[test]
    fn test_datatype_validation_pipelines() {
        use some_validation::pipeline::{
            BytesValidationPipeline, CollectionValidationPipeline, NumericValidationPipeline,
            StringValidationPipeline, ValidationPipeline,
        };

        // 1. Generic ValidationPipeline<T>
        let generic_pipeline = ValidationPipeline::<i32>::new()
            .step(|n| {
                if *n > 0 {
                    Ok(())
                } else {
                    Err(ValidationError::Custom("Must be positive".into()))
                }
            })
            .step(|n| {
                if *n < 100 {
                    Ok(())
                } else {
                    Err(ValidationError::Custom("Must be under 100".into()))
                }
            });
        assert!(generic_pipeline.validate(&50).is_ok());
        assert!(generic_pipeline.validate(&0).is_err());
        assert!(generic_pipeline.validate(&150).is_err());

        // 2. StringValidationPipeline
        let str_pipeline = StringValidationPipeline::new()
            .non_empty("code")
            .exact_length(6)
            .numeric_only();
        assert!(str_pipeline.validate("123456").is_ok());
        assert!(str_pipeline.validate("12345").is_err()); // too short
        assert!(str_pipeline.validate("12345a").is_err()); // not numeric
        assert!(str_pipeline.validate("").is_err()); // empty

        // 3. NumericValidationPipeline<i64>
        let num_pipeline = NumericValidationPipeline::<i64>::new()
            .min(10)
            .max(100)
            .not_expired(40);
        assert!(num_pipeline.validate(&50).is_ok());
        assert!(num_pipeline.validate(&5).is_err());
        assert!(num_pipeline.validate(&105).is_err());
        assert!(num_pipeline.validate(&30).is_err()); // expired relative to 40

        // 4. BytesValidationPipeline
        let bytes_pipeline = BytesValidationPipeline::new()
            .non_empty("key")
            .min_bytes(32)
            .max_bytes(64);
        let valid_key = [1u8; 32];
        assert!(bytes_pipeline.validate(&valid_key).is_ok());
        let short_key = [1u8; 16];
        assert!(bytes_pipeline.validate(&short_key).is_err());

        // 5. CollectionValidationPipeline
        let col_pipeline = CollectionValidationPipeline::<i32>::new()
            .non_empty("items")
            .min_len(2)
            .max_len(4)
            .all(|&x| x > 0, "All items must be positive");
        assert!(col_pipeline.validate(&[1, 2, 3]).is_ok());
        assert!(col_pipeline.validate(&[1]).is_err());
        assert!(col_pipeline.validate(&[1, -2]).is_err());
    }

    #[test]
    fn test_general_id_and_token_pipeline_validators() {
        use some_validation::pipeline::StringValidationPipeline;
        use some_validation::primitives::{
            validate_composite_token, validate_hex_digest, validate_mongodb_id,
            validate_mongodb_id_with_config, validate_numeric_code, validate_prefixed_uuid,
            validate_uuid, MongodbIdConfig,
        };

        // 1. validate_mongodb_id (24-char hex ObjectId)
        assert!(validate_mongodb_id("507f1f77bcf86cd799439011").is_ok());
        assert!(validate_mongodb_id("507F1F77BCF86CD799439011").is_ok());
        assert!(validate_mongodb_id("507f1f77bcf86cd79943901").is_err()); // 23 chars
        assert!(validate_mongodb_id("507f1f77bcf86cd79943901g").is_err()); // non-hex

        // 2. validate_mongodb_id_with_config (prefixed UUID or 24-char ObjectId)
        let doc_id_cfg = MongodbIdConfig::document_id();
        assert!(validate_mongodb_id_with_config(&doc_id_cfg, "507f1f77bcf86cd799439011").is_ok());
        assert!(validate_mongodb_id_with_config(&doc_id_cfg, "doc_99").is_ok());

        let usr_id_cfg = MongodbIdConfig::document_id().with_prefix("usr_");
        assert!(validate_mongodb_id_with_config(
            &usr_id_cfg,
            "usr_550e8400-e29b-41d4-a716-446655440000"
        )
        .is_ok());
        assert!(validate_mongodb_id_with_config(&usr_id_cfg, "507f1f77bcf86cd799439011").is_err());
        assert!(validate_mongodb_id_with_config(&usr_id_cfg, "invalid_id").is_err());

        // 3. validate_uuid & validate_prefixed_uuid
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_prefixed_uuid("chl_", "chl_550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(
            validate_prefixed_uuid("chl_", "usr_550e8400-e29b-41d4-a716-446655440000").is_err()
        );

        // 4. validate_numeric_code & validate_hex_digest
        assert!(validate_numeric_code("otp", 6, "482910").is_ok());
        assert!(validate_numeric_code("otp", 6, "48291a").is_err());
        assert!(validate_hex_digest("digest", 8, "a1b2c3d4").is_ok());
        assert!(validate_hex_digest("digest", 8, "a1b2c3d").is_err());

        // 5. validate_composite_token (zero-copy borrowed split)
        let (left, right) = validate_composite_token("token", '.', "chl_123.tok_456").unwrap();
        assert_eq!(left, "chl_123");
        assert_eq!(right, "tok_456");
        assert!(validate_composite_token("token", '.', "no_delimiter").is_err());
        assert!(validate_composite_token("token", '.', ".empty_left").is_err());
        assert!(validate_composite_token("token", '.', "empty_right.").is_err());

        // 6. StringValidationPipeline base32_only
        let base32_pipe = StringValidationPipeline::new()
            .non_empty("secret")
            .base32_only();
        assert!(base32_pipe.validate("JBSWY3DPEHPK3PXP").is_ok());
        assert!(base32_pipe.validate("JBSWY3DP89").is_err());
    }
}

#[test]
fn ng_identifier_configs_and_zero_copy_token_boundaries() {
    use some_validation::{
        validate_composite_token, validate_mongodb_id, validate_prefixed_uuid, MongodbIdConfig,
        ValidateWith, Validator,
    };
    let config = MongodbIdConfig::document_id()
        .with_prefix("usr_")
        .with_min_bytes(5)
        .with_max_bytes(8);
    for value in ["usr_a", "usr_1234"] {
        assert!(config.validate(value).is_ok());
        assert!(Validator::validate(&config, value).is_ok());
        assert!(value.validate_with(&config).is_ok());
    }
    for value in ["usr_", "usr_12345", "xxx_1", "usr_!", "usr_\n1", "usr_🔥"] {
        assert!(value.validate_with(&config).is_err(), "{value:?}");
    }
    for value in [
        "",
        "507f1f77bcf86cd79943901",
        "507f1f77bcf86cd7994390110",
        "507f1f77bcf86cd79943901g",
    ] {
        assert!(validate_mongodb_id(value).is_err());
    }
    assert!(validate_prefixed_uuid("usr_", "usr_not-a-uuid").is_err());
    assert!(validate_prefixed_uuid("usr_", "cch_550e8400-e29b-41d4-a716-446655440000").is_err());
    for value in ["", ":", "id:", ":secret", "id:secret:extra", "  :secret"] {
        assert!(
            validate_composite_token("token", ':', value).is_err(),
            "{value:?}"
        );
    }
    let raw = String::from("id:secret");
    let (id, secret) = validate_composite_token("token", ':', &raw).unwrap();
    assert_eq!(id.as_ptr(), raw.as_ptr());
    assert_eq!(secret.as_ptr(), raw[3..].as_ptr());
}

use ds_core::error::{ApiError, ApiResult};
use once_cell::sync::Lazy;
use regex::Regex;

// Compile regex patterns once at startup
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").expect("valid email regex")
});

/// Validate email format
pub fn validate_email(email: &str) -> ApiResult<()> {
    if email.is_empty() {
        return Err(ApiError::Unprocessable("email is required".into()));
    }

    if email.len() > 190 {
        return Err(ApiError::Unprocessable(
            "email too long (max 190 characters)".into(),
        ));
    }

    if !EMAIL_REGEX.is_match(email) {
        return Err(ApiError::Unprocessable("invalid email format".into()));
    }

    Ok(())
}

/// Validate password strength
pub fn validate_password(password: &str) -> ApiResult<()> {
    if password.len() < 8 {
        return Err(ApiError::Unprocessable(
            "password must be at least 8 characters".into(),
        ));
    }

    if password.len() > 128 {
        return Err(ApiError::Unprocessable(
            "password too long (max 128 characters)".into(),
        ));
    }

    // Check for at least one letter and one number (basic strength check)
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_letter || !has_number {
        return Err(ApiError::Unprocessable(
            "password must contain at least one letter and one number".into(),
        ));
    }

    Ok(())
}

/// Validate chat message content
pub fn validate_message_content(content: &str, max_length: usize) -> ApiResult<()> {
    if content.is_empty() {
        return Err(ApiError::Unprocessable(
            "message content cannot be empty".into(),
        ));
    }

    if content.len() > max_length {
        return Err(ApiError::Unprocessable(format!(
            "message too long (max {} characters)",
            max_length
        )));
    }

    Ok(())
}

/// Validate model name
pub fn validate_model_name(model: &str) -> ApiResult<()> {
    if model.trim().is_empty() {
        return Err(ApiError::Unprocessable("model name is required".into()));
    }

    if model.len() > 100 {
        return Err(ApiError::Unprocessable("model name too long".into()));
    }

    // Only allow alphanumeric, dash, underscore, colon (for Ollama model naming)
    if !model
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.')
    {
        return Err(ApiError::Unprocessable(
            "invalid characters in model name".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user+tag@example.co.uk").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email(&"a".repeat(200)).is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("P@ssw0rd!").is_ok());
    }

    #[test]
    fn test_validate_password_invalid() {
        assert!(validate_password("short1").is_err()); // too short
        assert!(validate_password("nodigits").is_err()); // no numbers
        assert!(validate_password("12345678").is_err()); // no letters
        assert!(validate_password(&"a1".repeat(100)).is_err()); // too long
    }

    #[test]
    fn test_validate_model_name_valid() {
        assert!(validate_model_name("llama3.2").is_ok());
        assert!(validate_model_name("mistral:7b").is_ok());
        assert!(validate_model_name("model_name-v2").is_ok());
    }

    #[test]
    fn test_validate_model_name_invalid() {
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name("   ").is_err());
        assert!(validate_model_name("model/with/slash").is_err());
        assert!(validate_model_name(&"a".repeat(150)).is_err());
    }
}

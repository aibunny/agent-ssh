use std::{borrow::Borrow, fmt};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ServerAlias(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProfileName(String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SignerName(String);

impl ServerAlias {
    pub fn new(value: &str) -> Result<Self, String> {
        validate_identifier(value).map(|_| Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ProfileName {
    pub fn new(value: &str) -> Result<Self, String> {
        validate_identifier(value).map(|_| Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SignerName {
    pub fn new(value: &str) -> Result<Self, String> {
        validate_identifier(value).map(|_| Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ServerAlias {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ProfileName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SignerName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ServerAlias {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Display for SignerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

const MAX_IDENTIFIER_LEN: usize = 64;

fn validate_identifier(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("identifier must not be empty".to_string());
    }

    if value.len() > MAX_IDENTIFIER_LEN {
        return Err(format!(
            "identifier must not exceed {MAX_IDENTIFIER_LEN} characters"
        ));
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err("identifier must not be empty".to_string());
    };

    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err("identifier must start with a lowercase ASCII letter or digit".to_string());
    }

    if chars.any(|char| !matches!(char, 'a'..='z' | '0'..='9' | '-' | '_')) {
        return Err(
            "identifier may contain only lowercase ASCII letters, digits, '-' and '_'".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MAX_IDENTIFIER_LEN, ProfileName, ServerAlias, SignerName};

    #[test]
    fn accepts_valid_identifiers() {
        assert!(ServerAlias::new("staging-api").is_ok());
        assert!(ProfileName::new("logs").is_ok());
        assert!(SignerName::new("step_ca").is_ok());
        assert!(ServerAlias::new("1server").is_ok());
        assert!(ServerAlias::new("a").is_ok());
    }

    #[test]
    fn rejects_empty_identifier() {
        assert!(ServerAlias::new("").is_err());
        assert!(ProfileName::new("").is_err());
    }

    #[test]
    fn rejects_uppercase_start() {
        assert!(ServerAlias::new("Staging").is_err());
    }

    #[test]
    fn rejects_special_characters() {
        assert!(ServerAlias::new("staging api").is_err());
        assert!(ServerAlias::new("staging!").is_err());
        assert!(ServerAlias::new("staging/api").is_err());
        assert!(ServerAlias::new("staging@api").is_err());
    }

    #[test]
    fn rejects_identifier_exceeding_max_length() {
        let long = "a".repeat(MAX_IDENTIFIER_LEN + 1);
        let err = ServerAlias::new(&long).expect_err("overlong identifier must be rejected");
        assert!(err.contains("exceed"), "error should mention length: {err}");
    }

    #[test]
    fn accepts_identifier_at_max_length() {
        let exact = "a".repeat(MAX_IDENTIFIER_LEN);
        assert!(ServerAlias::new(&exact).is_ok());
    }
}

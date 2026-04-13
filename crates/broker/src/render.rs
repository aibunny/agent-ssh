use std::collections::{BTreeMap, BTreeSet};

use agent_ssh_common::ProfileConfig;

use crate::BrokerError;

/// Maximum number of characters allowed in a single argument value.
/// Prevents resource exhaustion and avoids hitting OS command-line limits.
const MAX_ARG_VALUE_LEN: usize = 4096;

#[derive(Debug, Clone)]
pub struct CompiledProfile {
    placeholders: BTreeSet<String>,
    tokens: Vec<TemplateToken>,
}

#[derive(Debug, Clone)]
enum TemplateToken {
    Literal(String),
    Placeholder(String),
}

impl CompiledProfile {
    pub fn compile(profile_name: &str, profile: &ProfileConfig) -> Result<Self, BrokerError> {
        let mut tokens = Vec::new();
        let mut placeholders = BTreeSet::new();

        for token in profile.template.split_ascii_whitespace() {
            if let Some(placeholder) = parse_placeholder(token) {
                if !matches!(tokens.last(), Some(TemplateToken::Literal(previous)) if previous.starts_with('-'))
                {
                    return Err(BrokerError::UnsafeTemplate {
                        profile: profile_name.to_string(),
                        reason: format!(
                            "placeholder '{placeholder}' must follow a fixed option token that starts with '-'"
                        ),
                    });
                }

                if !placeholders.insert(placeholder.to_string()) {
                    return Err(BrokerError::UnsafeTemplate {
                        profile: profile_name.to_string(),
                        reason: format!("placeholder '{placeholder}' is declared more than once"),
                    });
                }
                tokens.push(TemplateToken::Placeholder(placeholder.to_string()));
                continue;
            }

            if !is_safe_literal_token(token) {
                return Err(BrokerError::UnsafeTemplate {
                    profile: profile_name.to_string(),
                    reason: format!(
                        "token '{token}' is outside the allowed shell-safe template grammar"
                    ),
                });
            }

            tokens.push(TemplateToken::Literal(token.to_string()));
        }

        if tokens.is_empty() {
            return Err(BrokerError::UnsafeTemplate {
                profile: profile_name.to_string(),
                reason: "template must not be empty".to_string(),
            });
        }

        Ok(Self {
            placeholders,
            tokens,
        })
    }

    pub fn render(
        &self,
        profile_name: &str,
        args: &BTreeMap<String, String>,
    ) -> Result<String, BrokerError> {
        for required in &self.placeholders {
            if !args.contains_key(required.as_str()) {
                return Err(BrokerError::MissingArgument {
                    profile: profile_name.to_string(),
                    name: required.clone(),
                });
            }
        }

        for supplied in args.keys() {
            if !self.placeholders.contains(supplied.as_str()) {
                return Err(BrokerError::UnexpectedArgument {
                    profile: profile_name.to_string(),
                    name: supplied.clone(),
                });
            }
        }

        let mut rendered = Vec::with_capacity(self.tokens.len());
        for token in &self.tokens {
            match token {
                TemplateToken::Literal(literal) => rendered.push(literal.clone()),
                TemplateToken::Placeholder(name) => {
                    let Some(value) = args.get(name.as_str()) else {
                        return Err(BrokerError::MissingArgument {
                            profile: profile_name.to_string(),
                            name: name.clone(),
                        });
                    };

                    // Reject control characters (includes null bytes, newlines, etc.)
                    if value.chars().any(char::is_control) {
                        return Err(BrokerError::InvalidArgumentValue {
                            profile: profile_name.to_string(),
                            name: name.clone(),
                        });
                    }

                    // Reject excessively long values to prevent command-line overflow.
                    if value.len() > MAX_ARG_VALUE_LEN {
                        return Err(BrokerError::InvalidArgumentValue {
                            profile: profile_name.to_string(),
                            name: name.clone(),
                        });
                    }

                    rendered.push(shell_escape(value));
                }
            }
        }

        Ok(rendered.join(" "))
    }
}

fn parse_placeholder(token: &str) -> Option<&str> {
    if token.starts_with("{{") && token.ends_with("}}") && token.len() > 4 {
        let inner = &token[2..token.len() - 2];
        if is_placeholder_name(inner) {
            return Some(inner);
        }
    }

    None
}

fn is_placeholder_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_lowercase() {
        return false;
    }

    chars.all(|char| matches!(char, 'a'..='z' | '0'..='9' | '_' | '-'))
}

fn is_safe_literal_token(token: &str) -> bool {
    token.chars().all(|char| {
        matches!(
            char,
            'a'..='z'
                | 'A'..='Z'
                | '0'..='9'
                | '_'
                | '.'
                | '/'
                | ':'
                | '='
                | '@'
                | '+'
                | '-'
                | ','
                | '%'
        )
    })
}

/// Single-quote escape suitable for POSIX shells.
/// Single quotes in the value are handled with the `'"'"'` splice.
fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let mut escaped = String::from("'");
    for char in value.chars() {
        if char == '\'' {
            escaped.push_str("'\"'\"'");
        } else {
            escaped.push(char);
        }
    }
    escaped.push('\'');
    escaped
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agent_ssh_common::ProfileConfig;

    use super::{CompiledProfile, MAX_ARG_VALUE_LEN};
    use crate::BrokerError;

    fn profile(template: &str) -> ProfileConfig {
        ProfileConfig {
            description: None,
            template: template.to_string(),
            requires_approval: false,
        }
    }

    // ── Template compilation ─────────────────────────────────────────────────

    #[test]
    fn rejects_unsafe_template_tokens() {
        let error =
            CompiledProfile::compile("logs", &profile("journalctl -u {{service}} | tail -n 10"))
                .expect_err("template with pipe should fail");

        match error {
            BrokerError::UnsafeTemplate { .. } => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_duplicate_placeholders() {
        let error = CompiledProfile::compile("logs", &profile("echo {{service}} {{service}}"))
            .expect_err("duplicate placeholders should fail");

        match error {
            BrokerError::UnsafeTemplate { .. } => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_placeholder_in_executable_position() {
        let error = CompiledProfile::compile("logs", &profile("{{cmd}} --version"))
            .expect_err("placeholder executable should fail");

        match error {
            BrokerError::UnsafeTemplate { .. } => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_placeholder_in_flag_position() {
        let error = CompiledProfile::compile("logs", &profile("tar {{flag}} /tmp/archive.tar"))
            .expect_err("placeholder flag should fail");

        match error {
            BrokerError::UnsafeTemplate { .. } => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_semicolon_in_literal() {
        CompiledProfile::compile("cmd", &profile("ls; rm -rf /"))
            .expect_err("semicolon must be rejected");
    }

    #[test]
    fn rejects_backtick_in_literal() {
        CompiledProfile::compile("cmd", &profile("echo `id`"))
            .expect_err("backtick must be rejected");
    }

    #[test]
    fn rejects_dollar_sign_in_literal() {
        CompiledProfile::compile("cmd", &profile("echo $HOME"))
            .expect_err("dollar sign must be rejected");
    }

    #[test]
    fn rejects_redirect_in_literal() {
        CompiledProfile::compile("cmd", &profile("cat /etc/passwd > /tmp/out"))
            .expect_err("redirect must be rejected");
    }

    #[test]
    fn rejects_ampersand_in_literal() {
        CompiledProfile::compile("cmd", &profile("sleep 100 &"))
            .expect_err("background operator must be rejected");
    }

    #[test]
    fn rejects_empty_template() {
        CompiledProfile::compile("empty", &profile("   "))
            .expect_err("whitespace-only template must be rejected");
    }

    #[test]
    fn rejects_malformed_placeholder_uppercase() {
        // Uppercase placeholder names are not allowed in the grammar.
        // The token "{{Service}}" should be treated as a literal and fail
        // the safe-literal check (curly braces are not in the allowed set).
        CompiledProfile::compile("cmd", &profile("echo {{Service}}"))
            .expect_err("uppercase placeholder must be rejected as unsafe literal");
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    #[test]
    fn renders_single_escaped_arguments() {
        let compiled = CompiledProfile::compile(
            "logs",
            &profile("journalctl -u {{service}} --since {{since}} --no-pager"),
        )
        .expect("template should compile");

        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());
        args.insert("since".to_string(), "10 min ago".to_string());

        let rendered = compiled.render("logs", &args).expect("should render");

        assert_eq!(
            rendered,
            "journalctl -u 'api' --since '10 min ago' --no-pager"
        );
    }

    #[test]
    fn renders_empty_argument_as_empty_quotes() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), String::new());

        let rendered = compiled.render("cmd", &args).expect("render");
        assert_eq!(rendered, "printf -- ''");
    }

    #[test]
    fn renders_value_containing_single_quote() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "it's a test".to_string());

        let rendered = compiled.render("cmd", &args).expect("render");
        // The single quote in "it's" must be escaped via the splice.
        assert_eq!(rendered, "printf -- 'it'\"'\"'s a test'");
    }

    #[test]
    fn rejects_control_character_in_argument() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "hello\x01world".to_string());

        compiled
            .render("cmd", &args)
            .expect_err("control char must be rejected");
    }

    #[test]
    fn rejects_null_byte_in_argument() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "hello\x00world".to_string());

        compiled
            .render("cmd", &args)
            .expect_err("null byte must be rejected");
    }

    #[test]
    fn rejects_newline_in_argument() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "line1\nline2".to_string());

        compiled
            .render("cmd", &args)
            .expect_err("newline must be rejected");
    }

    #[test]
    fn rejects_argument_value_exceeding_max_length() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "a".repeat(MAX_ARG_VALUE_LEN + 1));

        compiled
            .render("cmd", &args)
            .expect_err("overlong value must be rejected");
    }

    #[test]
    fn accepts_argument_value_at_max_length() {
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "a".repeat(MAX_ARG_VALUE_LEN));

        compiled
            .render("cmd", &args)
            .expect("value at exactly max length should be allowed");
    }

    #[test]
    fn rejects_missing_required_argument() {
        let compiled = CompiledProfile::compile(
            "logs",
            &profile("journalctl -u {{service}} --since {{since}}"),
        )
        .expect("compile");

        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());
        // "since" is missing.

        let err = compiled
            .render("logs", &args)
            .expect_err("missing arg must be rejected");
        match err {
            BrokerError::MissingArgument { name, .. } => {
                assert_eq!(name, "since");
            }
            other => panic!("expected MissingArgument, got: {other}"),
        }
    }

    #[test]
    fn rejects_unexpected_extra_argument() {
        let compiled = CompiledProfile::compile("cmd", &profile("df -h")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("extra".to_string(), "value".to_string());

        let err = compiled
            .render("cmd", &args)
            .expect_err("extra arg must be rejected");
        match err {
            BrokerError::UnexpectedArgument { name, .. } => {
                assert_eq!(name, "extra");
            }
            other => panic!("expected UnexpectedArgument, got: {other}"),
        }
    }

    // ── Shell-escape property tests ──────────────────────────────────────────

    #[test]
    fn shell_escaped_value_never_breaks_out_of_single_quotes() {
        // Verify that a value consisting entirely of single quotes is escaped
        // correctly and would not allow shell injection.
        //
        // Input value: ''' (three single quotes)
        // Each ' inside shell_escape is replaced with '"'"', so:
        //   open-' + '"'"' + '"'"' + '"'"' + close-' = ''"'"''"'"''"'"''
        let compiled =
            CompiledProfile::compile("cmd", &profile("printf -- {{msg}}")).expect("compile");

        let mut args = BTreeMap::new();
        args.insert("msg".to_string(), "'''".to_string());

        let rendered = compiled.render("cmd", &args).expect("render");
        assert_eq!(rendered, "printf -- ''\"'\"''\"'\"''\"'\"''");
    }
}

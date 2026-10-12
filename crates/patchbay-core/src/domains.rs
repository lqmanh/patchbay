use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    Domain,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainRule {
    pub id: String,
    pub value: String,
    pub kind: RuleKind,
    pub enabled: bool,
}

pub fn normalize_domain(input: &str) -> Option<String> {
    let lowered = input.trim().to_lowercase();
    if lowered.is_empty() {
        return None;
    }
    let mut value = strip_scheme(&lowered);
    value = value.split(['/', '?', '#']).next().unwrap_or_default();
    value = strip_port(value);
    value = value.strip_prefix("www.").unwrap_or(value);
    value = value.strip_suffix('.').unwrap_or(value);
    is_hostname(value).then(|| value.to_string())
}

pub fn regex_error(pattern: &str) -> Option<String> {
    if pattern.trim().is_empty() {
        return Some("pattern is empty".to_string());
    }
    if is_re2_incompatible(pattern) {
        return Some("lookarounds and backreferences are not supported by the daemon".to_string());
    }
    regex_syntax::parse(pattern)
        .err()
        .map(|_| "invalid regular expression".to_string())
}

pub fn create_rule(input: &str, kind: RuleKind) -> Option<DomainRule> {
    let value = match kind {
        RuleKind::Domain => normalize_domain(input)?,
        RuleKind::Regex => input.trim().to_string(),
    };
    if kind == RuleKind::Regex && regex_error(&value).is_some() {
        return None;
    }
    Some(DomainRule {
        id: new_rule_id(),
        value,
        kind,
        enabled: true,
    })
}

fn new_rule_id() -> String {
    let mut id = uuid::Uuid::new_v4().simple().to_string();
    id.truncate(8);
    id
}

fn strip_scheme(value: &str) -> &str {
    let Some(index) = value.find("://") else {
        return value;
    };
    let scheme = &value[..index];
    let mut chars = scheme.chars();
    let valid = matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '.' | '-'));
    if valid { &value[index + 3..] } else { value }
}

fn strip_port(value: &str) -> &str {
    let Some(index) = value.rfind(':') else {
        return value;
    };
    let port = &value[index + 1..];
    if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) {
        &value[..index]
    } else {
        value
    }
}

fn is_hostname(value: &str) -> bool {
    let mut labels = 0;
    for label in value.split('.') {
        labels += 1;
        if !is_label(label) {
            return false;
        }
    }
    labels >= 2
}

fn is_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn is_re2_incompatible(pattern: &str) -> bool {
    let lookaround = pattern.contains("(?=") || pattern.contains("(?!") || pattern.contains("(?<");
    let backreference = pattern
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'\\' && (b'1'..=b'9').contains(&pair[1]));
    lookaround || backreference
}

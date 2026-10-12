use patchbay_core::domains::{RuleKind, create_rule, normalize_domain, regex_error};

#[test]
fn normalizes_urls_ports_and_case() {
    assert_eq!(normalize_domain("  Reddit.COM "), Some("reddit.com".into()));
    assert_eq!(
        normalize_domain("https://www.reddit.com/r/programming"),
        Some("reddit.com".into())
    );
    assert_eq!(
        normalize_domain("sub.medium.com:8443"),
        Some("sub.medium.com".into())
    );
    assert_eq!(normalize_domain("example.org."), Some("example.org".into()));
}

#[test]
fn rejects_invalid_hosts() {
    assert_eq!(normalize_domain(""), None);
    assert_eq!(normalize_domain("single-label"), None);
    assert_eq!(normalize_domain("has space.com"), None);
    assert_eq!(normalize_domain("https://"), None);
}

#[test]
fn accepts_re2_safe_patterns() {
    assert_eq!(regex_error("^(.*\\.)?example\\.com$"), None);
}

#[test]
fn rejects_invalid_or_unsupported_patterns() {
    assert!(regex_error("(").is_some());
    assert!(regex_error("(?=foo)").is_some());
    assert!(regex_error("(a)\\1").is_some());
    assert!(regex_error("").is_some());
}

#[test]
fn creates_domain_rules_from_urls() {
    let rule = create_rule("https://www.medium.com/topic", RuleKind::Domain).expect("rule");
    assert_eq!(rule.value, "medium.com");
    assert_eq!(rule.kind, RuleKind::Domain);
    assert!(rule.enabled);
    assert_eq!(rule.id.len(), 8);
}

#[test]
fn creates_regex_rules_only_when_valid() {
    let rule = create_rule("^(.*\\.)?files\\.example\\.com$", RuleKind::Regex).expect("rule");
    assert_eq!(rule.kind, RuleKind::Regex);
    assert_eq!(create_rule("(?=bad)", RuleKind::Regex), None);
    assert_eq!(create_rule("not a domain", RuleKind::Domain), None);
}

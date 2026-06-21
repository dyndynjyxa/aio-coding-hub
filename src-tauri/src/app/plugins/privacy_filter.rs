//! Usage: Native Privacy Filter engine aligned with packyme/privacy-filter.

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use std::cmp::Ordering;
use std::sync::LazyLock;

const ENTROPY_MIN: f64 = 4.0;
const ENTROPY_MIN_STRICT: f64 = 4.8;
const CONTEXT_LOOKBACK: usize = 30;
const GITLEAKS_REGEX_SIZE_LIMIT: usize = 64 * 1024 * 1024;

static RE_EMAIL: LazyLock<Regex> =
    LazyLock::new(|| regex(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"));
static RE_PHONE_CN: LazyLock<Regex> = LazyLock::new(|| regex(r"(?:\+?86[-\s]?)?1[3-9][0-9]{9}"));
static RE_ID_CARD: LazyLock<Regex> = LazyLock::new(|| regex(r"[1-9][0-9]{16}[0-9Xx]"));
static RE_BANK_CARD: LazyLock<Regex> = LazyLock::new(|| regex(r"[0-9]{13,19}"));
static RE_IPV4: LazyLock<Regex> = LazyLock::new(|| {
    regex(r"(?:(?:25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivacyFilterEntity {
    pub(crate) entity_type: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivacyFilterResult {
    pub(crate) redacted: String,
    pub(crate) hit: bool,
    pub(crate) count: usize,
    pub(crate) entities: Vec<PrivacyFilterEntity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct PrivacyFilterStats {
    pub(crate) rules: usize,
    pub(crate) skipped: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PrivacyFilterOptions {
    allowed_labels: Option<std::collections::HashSet<&'static str>>,
}

impl PrivacyFilterOptions {
    pub(crate) fn from_sensitive_types(types: Option<&[String]>) -> Self {
        let Some(types) = types else {
            return Self::default();
        };

        let allowed_labels = types
            .iter()
            .filter_map(|item| sensitive_type_label(item.as_str()))
            .collect::<std::collections::HashSet<_>>();
        Self {
            allowed_labels: Some(allowed_labels),
        }
    }

    fn allows(&self, span: &Span) -> bool {
        self.allowed_labels
            .as_ref()
            .is_none_or(|labels| labels.contains(span.label))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PrivacyFilter {
    secrets: SecretDetector,
}

impl PrivacyFilter {
    pub(crate) fn from_gitleaks_toml(raw: &str) -> Result<Self, PrivacyFilterError> {
        Ok(Self {
            secrets: SecretDetector::from_gitleaks_toml(raw)?,
        })
    }

    #[cfg(test)]
    pub(crate) fn stats(&self) -> PrivacyFilterStats {
        PrivacyFilterStats {
            rules: self.secrets.rules.len(),
            skipped: self.secrets.skipped,
        }
    }

    #[cfg(test)]
    pub(crate) fn redact(&self, text: &str) -> PrivacyFilterResult {
        self.redact_with_options(text, &PrivacyFilterOptions::default())
    }

    pub(crate) fn redact_with_options(
        &self,
        text: &str,
        options: &PrivacyFilterOptions,
    ) -> PrivacyFilterResult {
        let mut spans = detect_pii(text);
        spans.extend(self.secrets.detect(text));
        spans.retain(|span| options.allows(span));

        let merged = merge_spans(spans);
        let mut redacted = String::with_capacity(text.len());
        let mut previous = 0usize;
        for span in &merged {
            redacted.push_str(&text[previous..span.start]);
            redacted.push_str(span.label);
            previous = span.end;
        }
        redacted.push_str(&text[previous..]);

        let entities = merged
            .iter()
            .map(|span| PrivacyFilterEntity {
                entity_type: span.label.to_string(),
                start: span.start,
                end: span.end,
                text: text[span.start..span.end].to_string(),
            })
            .collect::<Vec<_>>();

        PrivacyFilterResult {
            redacted,
            hit: !merged.is_empty(),
            count: merged.len(),
            entities,
        }
    }
}

fn sensitive_type_label(value: &str) -> Option<&'static str> {
    match value {
        "email" => Some("[邮箱]"),
        "cn_phone" | "phone" => Some("[电话]"),
        "cn_id_card" | "id_card" => Some("[身份证]"),
        "ip" | "ipv4" => Some("[IP]"),
        "bank_card" | "bank_card_candidate" => Some("[银行卡]"),
        "secret" | "token" | "api_key" | "openai_key" | "aws_access_key" | "github_token"
        | "google_api_key" | "slack_token" | "jwt" | "private_key" | "context_secret" => {
            Some("[密钥]")
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrivacyFilterError {
    message: String,
}

impl PrivacyFilterError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PrivacyFilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PrivacyFilterError {}

#[derive(Debug, Clone)]
struct Span {
    start: usize,
    end: usize,
    label: &'static str,
}

fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.retain(|span| span.start < span.end);
    spans.sort_by(|left, right| match left.start.cmp(&right.start) {
        Ordering::Equal => right.end.cmp(&left.end),
        other => other,
    });

    let mut merged = Vec::with_capacity(spans.len());
    let mut last_end = 0usize;
    let mut has_last = false;
    for span in spans {
        if !has_last || span.start >= last_end {
            last_end = span.end;
            has_last = true;
            merged.push(span);
        }
    }
    merged
}

fn regex(pattern: &str) -> Regex {
    RegexBuilder::new(pattern)
        .size_limit(GITLEAKS_REGEX_SIZE_LIMIT)
        .build()
        .expect("valid privacy filter regex")
}

fn detect_pii(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for mat in RE_EMAIL.find_iter(text) {
        let start = mat.start();
        let end = mat.end();
        if end < text.len()
            && text.as_bytes()[end] == b':'
            && end + 1 < text.len()
            && !matches!(text.as_bytes()[end + 1], b' ' | b'\t')
        {
            continue;
        }
        if is_in_ssh_command_context(text, start) {
            continue;
        }
        spans.push(Span {
            start,
            end,
            label: "[邮箱]",
        });
    }
    for mat in RE_PHONE_CN.find_iter(text) {
        if digit_bounded(text, mat.start(), mat.end()) {
            spans.push(Span {
                start: mat.start(),
                end: mat.end(),
                label: "[电话]",
            });
        }
    }
    for mat in RE_ID_CARD.find_iter(text) {
        if digit_bounded(text, mat.start(), mat.end()) {
            spans.push(Span {
                start: mat.start(),
                end: mat.end(),
                label: "[身份证]",
            });
        }
    }
    for mat in RE_IPV4.find_iter(text) {
        if ip_bounded(text, mat.start(), mat.end()) {
            spans.push(Span {
                start: mat.start(),
                end: mat.end(),
                label: "[IP]",
            });
        }
    }
    for mat in RE_BANK_CARD.find_iter(text) {
        if digit_bounded(text, mat.start(), mat.end()) && luhn_valid(mat.as_str()) {
            spans.push(Span {
                start: mat.start(),
                end: mat.end(),
                label: "[银行卡]",
            });
        }
    }
    spans
}

fn is_in_ssh_command_context(text: &str, email_start: usize) -> bool {
    let line_start = text[..email_start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line = &text[line_start..email_start];
    [
        "ssh ",
        "scp ",
        "rsync ",
        "sftp ",
        "ssh-copy-id ",
        "ssh-keygen ",
    ]
    .iter()
    .any(|command| line.contains(command))
}

fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

fn digit_bounded(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    if start > 0 && is_digit(bytes[start - 1]) {
        return false;
    }
    if end < bytes.len() && is_digit(bytes[end]) {
        return false;
    }
    true
}

fn ip_bounded(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    if start > 0 && (is_digit(bytes[start - 1]) || bytes[start - 1] == b'.') {
        return false;
    }
    if end < bytes.len() && (is_digit(bytes[end]) || bytes[end] == b'.') {
        return false;
    }
    true
}

fn luhn_valid(num: &str) -> bool {
    let mut sum = 0u32;
    let mut double = false;
    for byte in num.bytes().rev() {
        let mut digit = u32::from(byte - b'0');
        if double {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
        double = !double;
    }
    sum.is_multiple_of(10)
}

#[derive(Debug, Clone)]
struct SecretRule {
    regex: Regex,
    keywords: Vec<String>,
    entropy: f64,
    secret_group: usize,
}

#[derive(Debug, Clone)]
struct SecretDetector {
    rules: Vec<SecretRule>,
    skipped: usize,
    re_context_secret: Regex,
    re_entropy_token: Regex,
    re_secret_context: Regex,
    re_template_var: Regex,
    re_uuid: Regex,
    re_hex_only: Regex,
    re_auth_header_prefix: Regex,
    re_host_port_prefix: Regex,
}

#[derive(Debug, Deserialize)]
struct GitleaksConfig {
    #[serde(default)]
    rules: Vec<GitleaksRule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitleaksRule {
    #[allow(dead_code)]
    id: String,
    #[serde(default)]
    regex: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    entropy: f64,
    #[serde(default)]
    secret_group: usize,
}

impl SecretDetector {
    fn from_gitleaks_toml(raw: &str) -> Result<Self, PrivacyFilterError> {
        let config: GitleaksConfig = toml::from_str(raw).map_err(|err| {
            PrivacyFilterError::new(format!("failed to parse gitleaks rules: {err}"))
        })?;
        let mut detector = Self::empty();
        for rule in config.rules {
            let Some(pattern) = rule.regex else {
                continue;
            };
            let compiled = RegexBuilder::new(&pattern)
                .size_limit(GITLEAKS_REGEX_SIZE_LIMIT)
                .build();
            match compiled {
                Ok(regex) => detector.rules.push(SecretRule {
                    regex,
                    keywords: rule
                        .keywords
                        .into_iter()
                        .map(|keyword| keyword.to_ascii_lowercase())
                        .collect(),
                    entropy: rule.entropy,
                    secret_group: rule.secret_group,
                }),
                Err(err) => {
                    detector.skipped += 1;
                    tracing::warn!(error = %err, "skipping incompatible privacy-filter gitleaks rule");
                }
            }
        }
        if detector.rules.is_empty() {
            detector.load_builtin();
        }
        Ok(detector)
    }

    fn empty() -> Self {
        Self {
            rules: Vec::new(),
            skipped: 0,
            re_context_secret: regex(
                r#"(?i)(密码|口令|密钥|password|passwd|pwd|secret|token|api[_\s-]?key)\s*(?:是|为|:|：|=)\s*['"]?([^\s'"，。；;]{4,})"#,
            ),
            re_entropy_token: regex(r"[A-Za-z0-9+/=_\-]{20,}"),
            re_secret_context: regex(
                r"(?i)(?:password|passwd|pwd|secret|token|api[_\s-]?key|access[_\s-]?key|bearer|authorization|credential|jwt|密码|口令|密钥|凭证|令牌|鉴权)",
            ),
            re_template_var: regex(r"^(?:\{\{[^{}]+\}\}|\$\{[^{}]+\}|%\{[^{}]+\}|<[^<>]+>)$"),
            re_uuid: regex(
                r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
            ),
            re_hex_only: regex(r"^[0-9a-fA-F]+$"),
            re_auth_header_prefix: regex(
                r"(?i)\bauthorization\s*:\s*(?:basic|bearer|digest|ntlm|hmac|token)\s+$",
            ),
            re_host_port_prefix: regex(r"^[A-Za-z0-9][A-Za-z0-9.-]*\.[A-Za-z0-9-]+:"),
        }
    }

    fn load_builtin(&mut self) {
        for (pattern, keywords) in [
            (r"sk-(?:proj-)?[A-Za-z0-9_-]{20,}", vec!["sk-"]),
            (r"AKIA[0-9A-Z]{16}", vec!["akia"]),
            (
                r"gh[pousr]_[A-Za-z0-9]{36,}",
                vec!["ghp_", "gho_", "ghu_", "ghs_", "ghr_"],
            ),
            (r"AIza[0-9A-Za-z_-]{35}", vec!["aiza"]),
            (r"xox[baprs]-[0-9A-Za-z-]{10,}", vec!["xox"]),
            (
                r"eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
                vec!["eyj"],
            ),
            (r"-----BEGIN[A-Z ]*PRIVATE KEY-----", vec!["private key"]),
        ] {
            self.rules.push(SecretRule {
                regex: regex(pattern),
                keywords: keywords
                    .into_iter()
                    .map(|keyword| keyword.to_ascii_lowercase())
                    .collect(),
                entropy: 0.0,
                secret_group: 0,
            });
        }
    }

    fn detect(&self, text: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let lower = text.to_ascii_lowercase();

        for rule in &self.rules {
            if !rule_applies(rule, &lower) {
                continue;
            }
            for captures in rule.regex.captures_iter(text) {
                let Some(mut matched) = captures.get(0) else {
                    continue;
                };
                if rule.secret_group > 0 {
                    if let Some(group) = captures.get(rule.secret_group) {
                        matched = group;
                    }
                }
                let start = matched.start();
                let end = matched.end();
                if start >= end {
                    continue;
                }
                let candidate = &text[start..end];
                if rule.entropy > 0.0 && shannon_entropy(candidate) < rule.entropy {
                    continue;
                }
                if self.looks_like_url_match(candidate)
                    || self.is_template_var(candidate)
                    || self.is_hex_hash(candidate)
                    || self.is_uuid(candidate)
                    || is_business_id_assignment(candidate)
                    || is_likely_placeholder(candidate)
                    || has_json_noise(candidate)
                {
                    continue;
                }
                spans.push(Span {
                    start,
                    end,
                    label: "[密钥]",
                });
            }
        }

        for captures in self.re_context_secret.captures_iter(text) {
            let Some(value) = captures.get(2) else {
                continue;
            };
            let candidate = value.as_str();
            if self.is_template_var(candidate) {
                continue;
            }
            if candidate.len() <= 16 && shannon_entropy(candidate) < 3.0 {
                continue;
            }
            spans.push(Span {
                start: value.start(),
                end: value.end(),
                label: "[密钥]",
            });
        }

        for mat in self.re_entropy_token.find_iter(text) {
            let start = mat.start();
            let end = mat.end();
            let candidate = mat.as_str();

            let strong = self.has_strong_secret_context(text, start, end);
            if !strong && is_on_path_or_url_boundary(text, start, end) {
                continue;
            }
            if self.is_template_var(candidate)
                || self.is_hex_hash(candidate)
                || self.is_uuid(candidate)
                || is_business_id_assignment(candidate)
            {
                continue;
            }
            let threshold = if self.has_secret_context(text, start, end) {
                ENTROPY_MIN
            } else {
                ENTROPY_MIN_STRICT
            };
            if shannon_entropy(candidate) >= threshold {
                spans.push(Span {
                    start,
                    end,
                    label: "[密钥]",
                });
            }
        }

        spans
    }

    fn looks_like_url_match(&self, candidate: &str) -> bool {
        candidate.contains("://") || self.re_host_port_prefix.is_match(candidate)
    }

    fn is_template_var(&self, candidate: &str) -> bool {
        self.re_template_var.is_match(candidate)
    }

    fn is_uuid(&self, candidate: &str) -> bool {
        self.re_uuid.is_match(candidate)
    }

    fn is_hex_hash(&self, candidate: &str) -> bool {
        matches!(candidate.len(), 32 | 40 | 64) && self.re_hex_only.is_match(candidate)
    }

    fn has_secret_context(&self, text: &str, start: usize, end: usize) -> bool {
        let begin = floor_char_boundary(text, start.saturating_sub(CONTEXT_LOOKBACK));
        self.re_secret_context.is_match(&text[begin..end])
    }

    fn has_strong_secret_context(&self, text: &str, start: usize, end: usize) -> bool {
        let begin = floor_char_boundary(text, start.saturating_sub(CONTEXT_LOOKBACK));
        if self.re_auth_header_prefix.is_match(&text[begin..start]) {
            return true;
        }
        let region = &text[begin..end];
        let Some(last) = self.re_secret_context.find_iter(region).last() else {
            return false;
        };
        let candidate_start = start - begin;
        if last.start() >= candidate_start {
            return true;
        }
        region[last.end()..candidate_start].bytes().all(|byte| {
            matches!(
                byte,
                b' ' | b'\t' | b'\r' | b'\n' | b'=' | b':' | b'\'' | b'"'
            )
        })
    }
}

fn rule_applies(rule: &SecretRule, lower_text: &str) -> bool {
    rule.keywords.is_empty()
        || rule
            .keywords
            .iter()
            .any(|keyword| lower_text.contains(keyword))
}

fn is_on_path_or_url_boundary(text: &str, start: usize, end: usize) -> bool {
    let candidate = &text[start..end];
    if candidate.contains(['/', '\\', ':']) {
        return true;
    }
    let bytes = text.as_bytes();
    if start > 0 && b"/\\:.@?=".contains(&bytes[start - 1]) {
        return true;
    }
    if end < bytes.len() && b"/\\:.@?=".contains(&bytes[end]) {
        return true;
    }
    let begin = floor_char_boundary(text, start.saturating_sub(8));
    let lookback = &text[begin..start];
    [
        "http://", "https://", "ftp://", "ssh://", "s3://", "gs://", "oss://", "git@", "sha256:",
        "sha1:", "md5:",
    ]
    .iter()
    .any(|prefix| lookback.contains(prefix))
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn is_business_id_assignment(candidate: &str) -> bool {
    let Some(eq) = candidate.find('=') else {
        return false;
    };
    if eq == 0 {
        return false;
    }
    let name = candidate[..eq].to_ascii_lowercase();
    if ["key", "secret", "token", "auth", "password", "credential"]
        .iter()
        .any(|keyword| name.contains(keyword))
    {
        return false;
    }
    ["_id", "_uuid", "_uid", "_oid", "_no", "_seq"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn is_likely_placeholder(candidate: &str) -> bool {
    let upper = candidate.to_ascii_uppercase();
    [
        "REPLACE_ME",
        "REPLACE_THIS",
        "REPLACE_WITH",
        "YOUR_KEY",
        "YOUR_TOKEN",
        "YOUR_SECRET",
        "YOUR_API_KEY",
        "YOUR_PASSWORD",
        "INSERT_HERE",
        "INSERT_KEY",
        "INSERT_TOKEN",
        "PLACEHOLDER",
        "EXAMPLE_KEY",
        "EXAMPLE_TOKEN",
        "TODO",
        "FIXME",
        "XXXX",
    ]
    .iter()
    .any(|placeholder| upper.contains(placeholder))
}

fn has_json_noise(candidate: &str) -> bool {
    candidate.contains(',')
}

fn shannon_entropy(candidate: &str) -> f64 {
    if candidate.is_empty() {
        return 0.0;
    }
    let mut frequency = [0usize; 256];
    for byte in candidate.bytes() {
        frequency[usize::from(byte)] += 1;
    }
    let len = candidate.len() as f64;
    frequency
        .iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let probability = *count as f64 / len;
            -probability * probability.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter() -> PrivacyFilter {
        PrivacyFilter::from_gitleaks_toml(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/plugins/official/privacy-filter/rules/gitleaks.toml"
        )))
        .expect("privacy filter")
    }

    fn redact(filter: &PrivacyFilter, text: &str) -> String {
        filter.redact(text).redacted
    }

    #[test]
    fn privacy_filter_redacts_structured_pii_like_upstream() {
        let filter = filter();
        let output = redact(
            &filter,
            "邮箱 test.user@example.com 手机 13812345678 身份证 11010519900307743X IP 192.168.1.10",
        );

        assert!(output.contains("[邮箱]"));
        assert!(output.contains("[电话]"));
        assert!(output.contains("[身份证]"));
        assert!(output.contains("[IP]"));
        assert!(!output.contains("test.user@example.com"));
        assert!(!output.contains("13812345678"));
        assert!(!output.contains("11010519900307743X"));
        assert!(!output.contains("192.168.1.10"));
    }

    #[test]
    fn privacy_filter_uses_luhn_for_bank_cards() {
        let filter = filter();
        let valid = redact(&filter, "付款卡号 4111111111111111");
        let invalid = redact(&filter, "订单编号 1234567890123456");

        assert!(valid.contains("[银行卡]"));
        assert!(!valid.contains("4111111111111111"));
        assert_eq!(invalid, "订单编号 1234567890123456");
    }

    #[test]
    fn privacy_filter_skips_ssh_command_email_false_positives() {
        let filter = filter();
        for input in [
            "ssh user@host.example.com",
            "ssh -i ~/.ssh/id_rsa user@host.example.com",
            "打开 ssh user@host.example.com",
            "scp file.txt user@host.example.com:/data/",
            "rsync -av /src/ user@host.example.com:/dst/",
        ] {
            let output = redact(&filter, input);
            assert!(!output.contains("[邮箱]"), "input={input} output={output}");
        }

        assert!(redact(&filter, "我的邮箱是 alice@example.com 请保密").contains("[邮箱]"));
    }

    #[test]
    fn privacy_filter_redacts_context_and_entropy_secrets() {
        let filter = filter();
        for input in [
            "我的密码是 Hunter2xyz",
            "配置里 api_key = aB3xK9pLmN2qR7sT",
            "临时凭证 aB3xK9pLmN2qR7sT5vW1zY 已生成",
            "Authorization: Bearer abcDEF1234567890/xyzABC4567890==",
            "token=aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp",
        ] {
            let output = redact(&filter, input);
            assert!(output.contains("[密钥]"), "input={input} output={output}");
        }
    }

    #[test]
    fn privacy_filter_skips_common_entropy_false_positives() {
        let filter = filter();
        for input in [
            "ls /home/user/AbCdEfGh1234567890XyZ",
            "curl https://api.example.com/v1/users/AbCdEfGh1234567890XyZ",
            "aws s3 cp s3://my-bucket/dir/AbCdEfGh1234567890XyZ .",
            "docker pull registry.io/img@sha256:9f86d081884c7d659a2feaa0c55ad015b1b8a3e6b1d2c4a5e9f8b7d6c5a4b3210",
            "host=long-subdomain-with-many-chars.example.com",
            "订单编号 aB3xK9pLmN2qR7sT5vW1zY 已记账",
            "secret={{ API_KEY }} 或 token=${TOKEN}",
            "order_id=aB3xK9pLmN2qR7sT5vW1zY",
            "trace_id=550e8400-e29b-41d4-a716-446655440000",
            "commit 9f86d081884c7d659a2feaa0c55ad015b1b8a3e6b1d2c4a5e9f8b7d6c5a4b3210",
            "api_key.example.com/AbCdEfGh1234567890XyZ",
        ] {
            let output = redact(&filter, input);
            assert!(!output.contains("[密钥]"), "input={input} output={output}");
        }
    }

    #[test]
    fn privacy_filter_loads_gitleaks_rule_set() {
        let filter = filter();
        let stats = filter.stats();

        assert!(stats.rules > 100, "rules={}", stats.rules);
        assert_eq!(stats.skipped, 0);
    }

    #[test]
    fn privacy_filter_matches_upstream_regression_samples() {
        let filter = filter();
        let cases = [
            (
                "/Users/alice/Documents/notes/My Vault/00.索引/技术指南/密钥速查.md",
                false,
            ),
            (
                "/data/simulations/wind/20260528_sim_run/forward_run_2\"",
                false,
            ),
            ("/tmp/AbCdEfGh1234567890XyZQwErTyUiOp.log", false),
            (
                "https://example.com/files/AbCdEfGh1234567890XyZQwErTyUiOp",
                false,
            ),
            ("s3://bucket/aB3xK9pLmN2qR7sT5vW1zY/file.txt", false),
            (
                "registry.io/img@sha256:9f86d081884c7d659a2feaa0c55ad015b1b8a3e6b1d2c4a5e9f8b7d6c5a4b3210",
                false,
            ),
            ("token=aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp", true),
            ("api_key = aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp", true),
            (
                "Authorization: Bearer abcDEF1234567890/xyzABC4567890==",
                true,
            ),
            (
                "https://x.com/cb?token=aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp",
                true,
            ),
            (
                "https://x.com/open?file=/Users/alice/Documents/a.txt",
                false,
            ),
            ("order_id=aB3xK9pLmN2qR7sT5vW1zY", false),
            ("trace_id=550e8400-e29b-41d4-a716-446655440000", false),
            ("commit=9f86d081884c7d659a2feaa0c55ad015", false),
            ("secret={{ API_KEY }} 或 token=${TOKEN}", false),
            ("我的密码是 Hunter2xyz", true),
            ("\"密钥速查\"、config[\"api_key\"]", false),
            ("api_key.example.com/AbCdEfGh1234567890XyZ", false),
        ];

        for (input, expected_hit) in cases {
            let result = filter.redact(input);
            assert_eq!(
                result.hit, expected_hit,
                "input={input} output={}",
                result.redacted
            );
        }
    }
}

//! Usage: Security-sensitive helpers (token masking and constant-time equality).

const TOKEN_MASK_PREFIX_LEN: usize = 6;
const TOKEN_MASK_SUFFIX_LEN: usize = 4;

pub(crate) fn mask_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let len = trimmed.len();
    if len <= TOKEN_MASK_PREFIX_LEN + TOKEN_MASK_SUFFIX_LEN {
        return "*".repeat(len.min(8));
    }

    let prefix = &trimmed[..TOKEN_MASK_PREFIX_LEN];
    let suffix = &trimmed[len - TOKEN_MASK_SUFFIX_LEN..];
    format!("{prefix}...{suffix}")
}

pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, mask_token};

    #[test]
    fn mask_token_keeps_prefix_and_suffix() {
        let token = "abcdef1234567890";
        assert_eq!(mask_token(token), "abcdef...7890");
    }

    #[test]
    fn mask_token_short_values_redacts_fully() {
        assert_eq!(mask_token("abcd"), "****");
    }

    #[test]
    fn constant_time_eq_matches_exact_bytes() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"diff"));
    }
}

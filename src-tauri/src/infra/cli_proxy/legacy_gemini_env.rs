//! Ownership checks shared by native and WSL retirement of Gemini CLI's old .env.

pub(crate) fn legacy_env_assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let line = line
        .strip_prefix("export")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map(str::trim_start)
        .unwrap_or(line);
    let pos = line.find(['=', ':'])?;
    let (key, rest) = line.split_at(pos);
    let value = &rest[1..];
    // dotenv also supports `NAME: value`.
    if rest.starts_with(':') && !value.starts_with(char::is_whitespace) {
        return None;
    }
    Some((key.trim_end(), value.trim()))
}

pub(crate) fn legacy_env_owned_base<'a>(
    text: &'a str,
    injected: Option<&str>,
    placeholder: Option<&str>,
) -> Option<&'a str> {
    let values = |key| {
        text.lines()
            .filter_map(legacy_env_assignment)
            .filter_map(move |(k, v)| (k == key).then_some(v))
    };
    let mut bases = values("GOOGLE_GEMINI_BASE_URL");
    let base = bases.next()?;
    if !bases.all(|value| value == base) {
        return None;
    }
    if Some(base) == injected {
        return Some(base);
    }
    // A crash can leave an older port in the journal. Only the complete local AIO pair
    // is evidence of ownership; a new destination must never receive the backed-up key.
    let mut keys = values("GEMINI_API_KEY");
    let marker = placeholder?;
    (super::agy_gateway::is_local_gateway_url(base)
        && keys.next() == Some(marker)
        && keys.all(|value| value == marker))
    .then_some(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_port_recovery_requires_only_loopback_and_the_owned_key() {
        let journal = Some("http://127.0.0.1:1/gemini");
        for url in [
            "http://localhost:2/gemini",
            "http://[::1]:2/gemini",
            "http://127.0.0.1:2/gemini",
        ] {
            let env = format!("GOOGLE_GEMINI_BASE_URL={url}\nGEMINI_API_KEY=owned\n");
            assert_eq!(
                legacy_env_owned_base(&env, journal, Some("owned")),
                Some(url)
            );
            assert_eq!(
                legacy_env_owned_base(
                    &(env + "export GEMINI_API_KEY=custom\n"),
                    journal,
                    Some("owned")
                ),
                None
            );
        }
        for url in [
            "http://127.0.0.1.invalid:2/gemini",
            "https://127.0.0.1@vendor.example/gemini",
            "http://192.168.1.2:2/gemini",
            "http://localhost:2/other",
            "http://localhost:2/gemini?other=1",
        ] {
            let env = format!("GOOGLE_GEMINI_BASE_URL={url}\nGEMINI_API_KEY=owned\n");
            assert_eq!(legacy_env_owned_base(&env, journal, Some("owned")), None);
        }
    }
}

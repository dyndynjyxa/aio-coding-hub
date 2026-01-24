pub(super) fn normalize_repo_branch(branch: &str) -> String {
    let branch = branch.trim();
    if branch.is_empty() || branch.eq_ignore_ascii_case("auto") {
        "auto".to_string()
    } else {
        branch.to_string()
    }
}

pub(super) fn canonical_git_url_key(input: &str) -> String {
    let raw = input.trim();
    if raw.is_empty() {
        return String::new();
    }

    // scp-like: git@github.com:owner/repo(.git)
    if let Some(rest) = raw.strip_prefix("git@") {
        let (host, path) = match rest.split_once(':') {
            Some((host, path)) => (host, path),
            None => (rest, ""),
        };

        let host = host
            .trim()
            .trim_end_matches('/')
            .split(':')
            .next()
            .unwrap_or(host)
            .to_ascii_lowercase();
        let mut path = path.trim().trim_matches('/').to_string();
        if path.to_ascii_lowercase().ends_with(".git") {
            path.truncate(path.len().saturating_sub(4));
        }

        if path.is_empty() {
            return host;
        }

        return format!("{}/{}", host, path.to_ascii_lowercase());
    }

    // Strip scheme if present (https://, ssh://, git://, etc.)
    let mut rest = raw;
    if let Some(pos) = raw.find("://") {
        rest = &raw[(pos + 3)..];
    }

    // Strip userinfo (git@) when it appears before the first slash
    if let Some(at) = rest.find('@') {
        let slash = rest.find('/').unwrap_or(rest.len());
        if at < slash {
            rest = &rest[(at + 1)..];
        }
    }

    let rest = rest.trim().trim_matches('/');
    let (host, path) = match rest.split_once('/') {
        Some((host, path)) => (host, path),
        None => (rest, ""),
    };

    let host = host
        .trim()
        .trim_end_matches('/')
        .split(':')
        .next()
        .unwrap_or(host)
        .to_ascii_lowercase();

    let mut path = path.trim().trim_matches('/').to_string();
    if path.to_ascii_lowercase().ends_with(".git") {
        path.truncate(path.len().saturating_sub(4));
    }

    // Common user behavior: pasting GitHub browser URLs like /owner/repo/tree/main/...
    if host.ends_with("github.com") {
        let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segs.len() >= 2 {
            path = format!("{}/{}", segs[0], segs[1]);
        }
    }

    if path.is_empty() {
        host
    } else {
        format!("{}/{}", host, path.to_ascii_lowercase())
    }
}

pub(super) fn parse_github_owner_repo(input: &str) -> Option<(String, String)> {
    let key = canonical_git_url_key(input);
    if key.is_empty() {
        return None;
    }
    let (host, path) = key.split_once('/')?;
    if !host.ends_with("github.com") {
        return None;
    }
    let mut segs = path.split('/').filter(|s| !s.is_empty());
    let owner = segs.next()?.to_string();
    let repo = segs.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

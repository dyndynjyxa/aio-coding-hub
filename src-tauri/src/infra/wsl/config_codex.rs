//! WSL Codex CLI configuration.

use super::constants::{WSL_CODEX_API_KEY, WSL_CODEX_PROVIDER_KEY};
use super::detection::resolve_wsl_codex_home_host_path;
use super::manifest::WSL_CLIENT_CONFIG_MAX_BYTES;
use super::shell::{bash_single_quote, run_wsl_bash_script, wsl_resolve_codex_home_script};
use crate::shared::error::AppResult;
use crate::shared::fs::read_optional_file_with_max_len;

pub(super) fn build_wsl_codex_config(
    content: &str,
    proxy_origin: &str,
    supports_websockets: bool,
) -> AppResult<String> {
    let mut document = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| "failed to parse WSL Codex config.toml")?;
    document["preferred_auth_method"] = toml_edit::value("apikey");
    document["model_provider"] = toml_edit::value(WSL_CODEX_PROVIDER_KEY);
    let providers = document
        .entry("model_providers")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_like_mut()
        .ok_or("WSL Codex model_providers must be a table")?;
    let provider = providers
        .entry(WSL_CODEX_PROVIDER_KEY)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_like_mut()
        .ok_or("WSL Codex proxy provider must be a table")?;
    for (key, value) in [
        ("name", toml_edit::value(WSL_CODEX_PROVIDER_KEY)),
        ("base_url", toml_edit::value(format!("{proxy_origin}/v1"))),
        ("wire_api", toml_edit::value("responses")),
        ("requires_openai_auth", toml_edit::value(true)),
        ("supports_websockets", toml_edit::value(supports_websockets)),
    ] {
        provider.insert(key, value);
    }
    Ok(document.to_string())
}

pub(super) fn configure_wsl_codex(
    distro: &str,
    proxy_origin: &str,
    supports_websockets: bool,
) -> AppResult<()> {
    let home = resolve_wsl_codex_home_host_path(distro)?;
    std::fs::create_dir_all(&home).map_err(|e| format!("failed to create WSL Codex home: {e}"))?;
    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");
    for path in [&config_path, &auth_path] {
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Refusing to modify a symlinked WSL Codex config file".into());
        }
    }
    let original_config =
        read_optional_file_with_max_len(&config_path, WSL_CLIENT_CONFIG_MAX_BYTES)?;
    let original_auth = read_optional_file_with_max_len(&auth_path, WSL_CLIENT_CONFIG_MAX_BYTES)?;
    let content = std::str::from_utf8(original_config.as_deref().unwrap_or_default())
        .map_err(|_| "WSL Codex config.toml must be UTF-8")?;
    let config = build_wsl_codex_config(content, proxy_origin, supports_websockets)?;
    let mut auth = match original_auth.as_deref().filter(|bytes| !bytes.is_empty()) {
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(bytes)
            .map_err(|_| "failed to parse WSL Codex auth.json")?,
        None => serde_json::json!({}),
    };
    auth.as_object_mut()
        .ok_or("WSL Codex auth.json must be an object")?
        .insert(
            "OPENAI_API_KEY".to_string(),
            serde_json::json!(WSL_CODEX_API_KEY),
        );
    let auth_content = serde_json::to_string_pretty(&auth)
        .map_err(|e| format!("failed to serialize WSL Codex auth.json: {e}"))?
        + "\n";
    // Keep writes inside the distro so replacement preserves Linux permissions and
    // a failed rename never removes the previous file.
    let script = format!(
        r#"
set -euo pipefail
aio_user_home="$(getent passwd "$(whoami)" | cut -d: -f6)"
{resolver}
mkdir -p "$codex_home"
config_path="$codex_home/config.toml"
auth_path="$codex_home/auth.json"
if [ -L "$config_path" ] || [ -L "$auth_path" ]; then
  echo "Refusing to modify a symlinked WSL Codex config file" >&2
  exit 2
fi
ts="$(date +%s)"
[ ! -f "$config_path" ] || cp -a "$config_path" "$config_path.bak.$ts"
[ ! -f "$auth_path" ] || cp -a "$auth_path" "$auth_path.bak.$ts"
tmp_config="$(mktemp "${{config_path}}.tmp.XXXXXX")"
tmp_auth="$(mktemp "${{auth_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_config" "$tmp_auth"; }}
trap cleanup EXIT
printf '%s' {config} > "$tmp_config"
printf '%s' {auth} > "$tmp_auth"
if [ -f "$config_path" ]; then
  chmod --reference="$config_path" "$tmp_config"
fi
if [ -f "$auth_path" ]; then
  chmod --reference="$auth_path" "$tmp_auth"
fi
mv -f "$tmp_config" "$config_path"
if mv -f "$tmp_auth" "$auth_path"; then
  exit 0
fi
if [ -f "$config_path.bak.$ts" ]; then
  cp -a "$config_path.bak.$ts" "$config_path" || {{ echo "WSL Codex config rollback failed" >&2; exit 1; }}
else
  rm -f "$config_path" || {{ echo "WSL Codex config rollback failed" >&2; exit 1; }}
fi
echo "WSL Codex auth write failed; config rolled back" >&2
exit 1
"#,
        resolver = wsl_resolve_codex_home_script("codex_home").replace("$HOME", "$aio_user_home"),
        config = bash_single_quote(&config),
        auth = bash_single_quote(&auth_content),
    );
    run_wsl_bash_script(distro, &script)
}

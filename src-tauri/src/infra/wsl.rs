//! Usage: Windows WSL detection and per-distro client configuration helpers.

use crate::settings;
use serde::Serialize;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
pub struct WslDetection {
    pub detected: bool,
    pub distros: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslDistroConfigStatus {
    pub distro: String,
    pub claude: bool,
    pub codex: bool,
    pub gemini: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureCliReport {
    pub cli_key: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureDistroReport {
    pub distro: String,
    pub ok: bool,
    pub results: Vec<WslConfigureCliReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WslConfigureReport {
    pub ok: bool,
    pub message: String,
    pub distros: Vec<WslConfigureDistroReport>,
}

#[cfg(windows)]
fn hide_window_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn hide_window_cmd(program: &str) -> Command {
    Command::new(program)
}

fn decode_utf16_le(mut bytes: &[u8]) -> String {
    // BOM (FF FE)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        bytes = &bytes[2..];
    }

    let len = bytes.len() - (bytes.len() % 2);
    let mut u16s = Vec::with_capacity(len / 2);
    for chunk in bytes[..len].chunks_exact(2) {
        u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    String::from_utf16_lossy(&u16s)
}

fn bash_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

pub fn detect() -> WslDetection {
    let mut out = WslDetection {
        detected: false,
        distros: Vec::new(),
    };

    if !cfg!(windows) {
        return out;
    }

    let output = hide_window_cmd("wsl").args(["--list", "--quiet"]).output();
    let Ok(output) = output else {
        return out;
    };
    if !output.status.success() {
        return out;
    }

    let decoded = decode_utf16_le(&output.stdout);
    for line in decoded.lines() {
        let mut distro = line.trim().to_string();
        distro = distro.trim_matches(&['\0', '\r'][..]).trim().to_string();
        if distro.is_empty() {
            continue;
        }
        if distro.starts_with("Windows") {
            continue;
        }
        out.distros.push(distro);
    }

    out.detected = !out.distros.is_empty();
    out
}

pub fn host_ipv4_best_effort() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }

    let output = hide_window_cmd("ipconfig").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    use std::net::Ipv4Addr;

    let mut in_wsl_adapter = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.contains("vEthernet (WSL)")
            || line.contains("vEthernet(WSL)")
            || line.contains("Ethernet adapter vEthernet (WSL)")
        {
            in_wsl_adapter = true;
            continue;
        }

        // "adapter" boundary (English output). If localized, we keep scanning until we see IPv4.
        if in_wsl_adapter && line.contains("adapter") && !line.contains("WSL") {
            break;
        }

        if !in_wsl_adapter {
            continue;
        }

        if line.contains("IPv4") || line.contains("IP Address") {
            let (_, tail) = line.rsplit_once(':')?;
            let ip = tail.trim();
            if ip.is_empty() || ip.contains(':') {
                continue;
            }
            if ip.parse::<Ipv4Addr>().is_ok() {
                return Some(ip.to_string());
            }
        }
    }

    None
}

fn run_wsl_bash_script(distro: &str, script: &str) -> Result<(), String> {
    let mut cmd = hide_window_cmd("wsl");
    cmd.args(["-d", distro, "bash"]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn wsl: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("failed to write wsl stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for wsl: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(if msg.is_empty() {
        "unknown error".to_string()
    } else {
        msg
    })
}

fn configure_wsl_claude(distro: &str, proxy_origin: &str) -> Result<(), String> {
    let base_url = format!("{proxy_origin}/claude");
    let base_url = bash_single_quote(&base_url);
    let auth_token = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

mkdir -p "$HOME/.claude"
config_path="$HOME/.claude/settings.json"

if [ -L "$config_path" ]; then
  echo "Refusing to modify: $config_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

base_url={base_url}
auth_token={auth_token}

ts="$(date +%s)"
if [ -f "$config_path" ]; then
  cp -a "$config_path" "$config_path.bak.$ts"
fi

tmp_path="$(mktemp "${{config_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

if command -v jq >/dev/null 2>&1; then
  if [ -s "$config_path" ]; then
    if ! jq -e 'type=="object" and (.env==null or (.env|type)=="object")' "$config_path" >/dev/null; then
      echo "Refusing to modify: $config_path must be a JSON object and env must be an object (or null)." >&2
      exit 2
    fi

    jq --arg base_url "$base_url" --arg auth_token "$auth_token" '
      .env = (.env // {{}})
      | .env.ANTHROPIC_BASE_URL = $base_url
      | .env.ANTHROPIC_AUTH_TOKEN = $auth_token
    ' "$config_path" > "$tmp_path"
  else
    jq -n --arg base_url "$base_url" --arg auth_token "$auth_token" '{{env:{{ANTHROPIC_BASE_URL:$base_url, ANTHROPIC_AUTH_TOKEN:$auth_token}}}}' > "$tmp_path"
  fi

  jq -e --arg base_url "$base_url" --arg auth_token "$auth_token" '
    .env.ANTHROPIC_BASE_URL == $base_url and .env.ANTHROPIC_AUTH_TOKEN == $auth_token
  ' "$tmp_path" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
  python3 - "$base_url" "$auth_token" "$config_path" "$tmp_path" <<'PY'
import json
import sys
from pathlib import Path

base_url, auth_token, src, dst = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]

data = {{}}
try:
    text = Path(src).read_text(encoding="utf-8")
    if text.strip():
        data = json.loads(text)
except FileNotFoundError:
    data = {{}}
except Exception as e:
    sys.stderr.write(f"Failed to parse existing settings.json: {{e}}\\n")
    sys.exit(2)

if not isinstance(data, dict):
    sys.stderr.write("settings.json must be a JSON object\\n")
    sys.exit(2)

env = data.get("env")
if env is None:
    env = {{}}
if not isinstance(env, dict):
    sys.stderr.write("settings.json env must be a JSON object\\n")
    sys.exit(2)

env["ANTHROPIC_BASE_URL"] = base_url
env["ANTHROPIC_AUTH_TOKEN"] = auth_token
data["env"] = env

Path(dst).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\\n", encoding="utf-8")
PY

  python3 - "$base_url" "$auth_token" "$tmp_path" <<'PY'
import json
import sys
from pathlib import Path

base_url, auth_token, path = sys.argv[1], sys.argv[2], sys.argv[3]
payload = json.loads(Path(path).read_text(encoding="utf-8"))
ok = (
    isinstance(payload, dict)
    and isinstance(payload.get("env"), dict)
    and payload["env"].get("ANTHROPIC_BASE_URL") == base_url
    and payload["env"].get("ANTHROPIC_AUTH_TOKEN") == auth_token
)
if not ok:
    sys.stderr.write("Sanity check failed for generated settings.json\\n")
    sys.exit(2)
PY
else
  if [ -s "$config_path" ]; then
    echo "Missing jq/python3; cannot safely merge existing $config_path" >&2
    exit 2
  fi

  cat > "$tmp_path" <<EOF
{{
  "env": {{
    "ANTHROPIC_BASE_URL": "$base_url",
    "ANTHROPIC_AUTH_TOKEN": "$auth_token"
  }}
}}
EOF
fi

if [ ! -s "$tmp_path" ]; then
  echo "Sanity check failed: generated settings.json is empty" >&2
  exit 2
fi

if [ -f "$config_path" ]; then
  chmod --reference="$config_path" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$config_path"
trap - EXIT
"#
    );

    run_wsl_bash_script(distro, &script)
}

fn configure_wsl_codex(distro: &str, proxy_origin: &str) -> Result<(), String> {
    let base_url = format!("{proxy_origin}/v1");
    let base_url = bash_single_quote(&base_url);
    let provider_key = bash_single_quote("aio");
    let api_key = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

mkdir -p "$HOME/.codex"
config_path="$HOME/.codex/config.toml"
auth_path="$HOME/.codex/auth.json"

if [ -L "$config_path" ]; then
  echo "Refusing to modify: $config_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi
if [ -L "$auth_path" ]; then
  echo "Refusing to modify: $auth_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

base_url={base_url}
provider_key={provider_key}
api_key={api_key}

ts="$(date +%s)"
[ -f "$config_path" ] && cp -a "$config_path" "$config_path.bak.$ts"
[ -f "$auth_path" ] && cp -a "$auth_path" "$auth_path.bak.$ts"

tmp_config="$(mktemp "${{config_path}}.tmp.XXXXXX")"
tmp_auth="$(mktemp "${{auth_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_config" "$tmp_auth"; }}
trap cleanup EXIT

if [ -s "$config_path" ]; then
  awk -v provider_key="$provider_key" -v base_url="$base_url" '
    BEGIN {{ in_root=1; seen_pref=0; seen_model=0; skipping=0 }}
    function ltrim(s) {{ sub(/^[[:space:]]+/, "", s); return s }}
    function rtrim(s) {{ sub(/[[:space:]]+$/, "", s); return s }}
    function extract_header(s) {{
      if (match(s, /^\[[^\]]+\]/)) {{
        return substr(s, RSTART, RLENGTH)
      }}
      return s
    }}
    function is_target_section(h, pk) {{
      header = extract_header(h)
      base1 = "[model_providers." pk "]"
      base2 = "[model_providers.\"" pk "\"]"
      base3 = "[model_providers.'"'"'" pk "'"'"']"
      prefix1 = "[model_providers." pk "."
      prefix2 = "[model_providers.\"" pk "\"."
      prefix3 = "[model_providers.'"'"'" pk "'"'"'."
      return (header == base1 || header == base2 || header == base3 || index(header, prefix1) == 1 || index(header, prefix2) == 1 || index(header, prefix3) == 1)
    }}
    {{
      line=$0
      trimmed=rtrim(ltrim(line))

      # skipping check BEFORE comment check to delete comments inside skipped section
      if (skipping) {{
        if (substr(trimmed, 1, 1) == "[") {{
          if (is_target_section(trimmed, provider_key)) {{
            next
          }}
          skipping=0
        }} else {{
          next
        }}
      }}

      if (trimmed ~ /^#/) {{ print line; next }}

      if (in_root && substr(trimmed, 1, 1) == "[") {{
        inserted=0
        if (!seen_pref) {{ print "preferred_auth_method = \"apikey\""; seen_pref=1; inserted=1 }}
        if (!seen_model) {{ print "model_provider = \"" provider_key "\""; seen_model=1; inserted=1 }}
        if (inserted) print ""
        in_root=0
      }}

      if (is_target_section(trimmed, provider_key)) {{
        skipping=1
        next
      }}

      if (in_root && trimmed ~ /^preferred_auth_method[[:space:]]*=/) {{
        if (!seen_pref) {{ print "preferred_auth_method = \"apikey\""; seen_pref=1 }}
        next
      }}
      if (in_root && trimmed ~ /^model_provider[[:space:]]*=/) {{
        if (!seen_model) {{ print "model_provider = \"" provider_key "\""; seen_model=1 }}
        next
      }}

      print line
    }}
    END {{
      if (in_root) {{
        if (!seen_pref) print "preferred_auth_method = \"apikey\""
        if (!seen_model) print "model_provider = \"" provider_key "\""
      }}
      print ""
      print "[model_providers." provider_key "]"
      print "name = \"" provider_key "\""
      print "base_url = \"" base_url "\""
      print "wire_api = \"responses\""
      print "requires_openai_auth = true"
    }}
  ' "$config_path" > "$tmp_config"
else
  cat > "$tmp_config" <<EOF
preferred_auth_method = "apikey"
model_provider = "$provider_key"

[model_providers.$provider_key]
name = "$provider_key"
base_url = "$base_url"
wire_api = "responses"
requires_openai_auth = true
EOF
fi

if [ ! -s "$tmp_config" ]; then
  echo "Sanity check failed: generated config.toml is empty" >&2
  exit 2
fi
grep -qF 'preferred_auth_method = "apikey"' "$tmp_config" || {{ echo "Sanity check failed: missing preferred_auth_method" >&2; exit 2; }}
grep -qF "model_provider = \"$provider_key\"" "$tmp_config" || {{ echo "Sanity check failed: missing model_provider" >&2; exit 2; }}
grep -qF "base_url = \"$base_url\"" "$tmp_config" || {{ echo "Sanity check failed: missing provider base_url" >&2; exit 2; }}

count_section="$(awk -v pk="$provider_key" '
  BEGIN {{ c=0 }}
  function extract_header(s) {{
    if (match(s, /^\[[^\]]+\]/)) {{
      return substr(s, RSTART, RLENGTH)
    }}
    return s
  }}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    sub(/[[:space:]]+$/, "", line)
    if (line ~ /^#/) next
    if (substr(line, 1, 1) != "[") next
    header = extract_header(line)
    base1 = "[model_providers." pk "]"
    base2 = "[model_providers.\"" pk "\"]"
    base3 = "[model_providers.'"'"'" pk "'"'"']"
    if (header == base1 || header == base2 || header == base3) c++
  }}
  END {{ print c }}
' "$tmp_config")"
if [ "$count_section" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one [model_providers.$provider_key] section, got $count_section" >&2
  exit 2
fi

if command -v jq >/dev/null 2>&1; then
  if [ -s "$auth_path" ]; then
    if ! jq -e 'type=="object"' "$auth_path" >/dev/null; then
      echo "Refusing to modify: $auth_path must be a JSON object." >&2
      exit 2
    fi
    jq --arg api_key "$api_key" '.OPENAI_API_KEY = $api_key' "$auth_path" > "$tmp_auth"
  else
    jq -n --arg api_key "$api_key" '{{OPENAI_API_KEY:$api_key}}' > "$tmp_auth"
  fi
  jq -e --arg api_key "$api_key" '.OPENAI_API_KEY == $api_key' "$tmp_auth" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
  python3 - "$api_key" "$auth_path" "$tmp_auth" <<'PY'
import json
import sys
from pathlib import Path

api_key, src, dst = sys.argv[1], sys.argv[2], sys.argv[3]
data = {{}}
try:
    text = Path(src).read_text(encoding="utf-8")
    if text.strip():
        data = json.loads(text)
except FileNotFoundError:
    data = {{}}
except Exception as e:
    sys.stderr.write(f"Failed to parse existing auth.json: {{e}}\n")
    sys.exit(2)

if not isinstance(data, dict):
    sys.stderr.write("auth.json must be a JSON object\n")
    sys.exit(2)

data["OPENAI_API_KEY"] = api_key
Path(dst).write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY
else
  if [ -s "$auth_path" ]; then
    echo "Missing jq/python3; cannot safely merge existing $auth_path" >&2
    exit 2
  fi
  cat > "$tmp_auth" <<EOF
{{"OPENAI_API_KEY":"$api_key"}}
EOF
fi

if [ ! -s "$tmp_auth" ]; then
  echo "Sanity check failed: generated auth.json is empty" >&2
  exit 2
fi
grep -qF '"OPENAI_API_KEY"' "$tmp_auth" || {{ echo "Sanity check failed: missing OPENAI_API_KEY" >&2; exit 2; }}

if [ -f "$config_path" ]; then
  chmod --reference="$config_path" "$tmp_config" 2>/dev/null || true
fi
if [ -f "$auth_path" ]; then
  chmod --reference="$auth_path" "$tmp_auth" 2>/dev/null || true
fi

if mv -f "$tmp_config" "$config_path"; then
  if mv -f "$tmp_auth" "$auth_path"; then
    trap - EXIT
    exit 0
  fi

  echo "Failed to write $auth_path; attempting to rollback $config_path" >&2
  if [ -f "$config_path.bak.$ts" ]; then
    if cp -a "$config_path.bak.$ts" "$config_path"; then
      echo "Rollback successful: restored $config_path from backup" >&2
    else
      echo "CRITICAL: Rollback failed! Manual recovery needed: cp $config_path.bak.$ts $config_path" >&2
    fi
  else
    echo "WARNING: No backup found for $config_path, moving to $config_path.failed.$ts" >&2
    mv -f "$config_path" "$config_path.failed.$ts" 2>/dev/null || echo "CRITICAL: Failed to move config to .failed" >&2
  fi
  exit 1
fi

echo "Failed to write $config_path" >&2
exit 1
"#
    );

    run_wsl_bash_script(distro, &script)
}

fn configure_wsl_gemini(distro: &str, proxy_origin: &str) -> Result<(), String> {
    let base_url = format!("{proxy_origin}/gemini");
    let base_url = bash_single_quote(&base_url);
    let api_key = bash_single_quote("aio-coding-hub");

    let script = format!(
        r#"
set -euo pipefail

HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

mkdir -p "$HOME/.gemini"
env_path="$HOME/.gemini/.env"

if [ -L "$env_path" ]; then
  echo "Refusing to modify: $env_path is a symlink. Please manage it manually or remove the symlink first." >&2
  exit 2
fi

gemini_base_url={base_url}
api_key={api_key}

ts="$(date +%s)"
[ -f "$env_path" ] && cp -a "$env_path" "$env_path.bak.$ts"

tmp_path="$(mktemp "${{env_path}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

if [ -f "$env_path" ]; then
  awk -v gemini_base_url="$gemini_base_url" -v api_key="$api_key" '
    BEGIN {{ seen_base=0; seen_key=0 }}
    function ltrim(s) {{ sub(/^[[:space:]]+/, "", s); return s }}
    {{
      line=$0
      trimmed=ltrim(line)
      if (trimmed ~ /^#/) {{ print line; next }}

      prefix=""
      rest=trimmed
      if (rest ~ /^export[[:space:]]+/) {{
        prefix="export "
        sub(/^export[[:space:]]+/, "", rest)
      }}

      if (rest ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) {{
        if (!seen_base) {{
          print prefix "GOOGLE_GEMINI_BASE_URL=" gemini_base_url
          seen_base=1
        }}
        next
      }}
      if (rest ~ /^GEMINI_API_KEY[[:space:]]*=/) {{
        if (!seen_key) {{
          print prefix "GEMINI_API_KEY=" api_key
          seen_key=1
        }}
        next
      }}

      print line
    }}
    END {{
      if (!seen_base) print "GOOGLE_GEMINI_BASE_URL=" gemini_base_url
      if (!seen_key) print "GEMINI_API_KEY=" api_key
    }}
  ' "$env_path" > "$tmp_path"
else
  cat > "$tmp_path" <<EOF
GOOGLE_GEMINI_BASE_URL=$gemini_base_url
GEMINI_API_KEY=$api_key
EOF
fi

if [ ! -s "$tmp_path" ]; then
  echo "Sanity check failed: generated .env is empty" >&2
  exit 2
fi

count_base="$(awk '
  BEGIN{{c=0}}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) c++
  }}
  END{{print c}}
' "$tmp_path")"
if [ "$count_base" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one GOOGLE_GEMINI_BASE_URL, got $count_base" >&2
  exit 2
fi

count_key="$(awk '
  BEGIN{{c=0}}
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GEMINI_API_KEY[[:space:]]*=/) c++
  }}
  END{{print c}}
' "$tmp_path")"
if [ "$count_key" -ne 1 ]; then
  echo "Sanity check failed: expected exactly one GEMINI_API_KEY, got $count_key" >&2
  exit 2
fi

actual_base="$(awk '
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/) {{
      sub(/^GOOGLE_GEMINI_BASE_URL[[:space:]]*=/, "", line)
      sub(/[[:space:]]+$/, "", line)
      print line
      exit
    }}
  }}
' "$tmp_path")"
if [ "$actual_base" != "$gemini_base_url" ]; then
  echo "Sanity check failed: GOOGLE_GEMINI_BASE_URL mismatch" >&2
  exit 2
fi

actual_key="$(awk '
  {{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line ~ /^#/) next
    if (line ~ /^export[[:space:]]+/) sub(/^export[[:space:]]+/, "", line)
    if (line ~ /^GEMINI_API_KEY[[:space:]]*=/) {{
      sub(/^GEMINI_API_KEY[[:space:]]*=/, "", line)
      sub(/[[:space:]]+$/, "", line)
      print line
      exit
    }}
  }}
' "$tmp_path")"
if [ "$actual_key" != "$api_key" ]; then
  echo "Sanity check failed: GEMINI_API_KEY mismatch" >&2
  exit 2
fi

if [ -f "$env_path" ]; then
  chmod --reference="$env_path" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$env_path"
trap - EXIT
"#
    );

    run_wsl_bash_script(distro, &script)
}

pub fn get_config_status(distros: &[String]) -> Vec<WslDistroConfigStatus> {
    if !cfg!(windows) {
        return Vec::new();
    }

    let mut out = Vec::new();
    for distro in distros {
        let claude = hide_window_cmd("wsl")
            .args([
                "-d",
                distro,
                "bash",
                "-lc",
                "test -f ~/.claude/settings.json",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let codex = hide_window_cmd("wsl")
            .args(["-d", distro, "bash", "-lc", "test -f ~/.codex/config.toml"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let gemini = hide_window_cmd("wsl")
            .args(["-d", distro, "bash", "-lc", "test -f ~/.gemini/.env"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        out.push(WslDistroConfigStatus {
            distro: distro.clone(),
            claude,
            codex,
            gemini,
        });
    }

    out
}

pub fn configure_clients(
    distros: &[String],
    targets: &settings::WslTargetCli,
    proxy_origin: &str,
) -> WslConfigureReport {
    if !cfg!(windows) {
        return WslConfigureReport {
            ok: false,
            message: "WSL configuration is only available on Windows".to_string(),
            distros: Vec::new(),
        };
    }

    let mut distro_reports = Vec::new();
    let mut success_ops = 0usize;
    let mut error_ops = 0usize;

    for distro in distros {
        let mut results = Vec::new();

        if targets.claude {
            match configure_wsl_claude(distro, proxy_origin) {
                Ok(()) => results.push(WslConfigureCliReport {
                    cli_key: "claude".to_string(),
                    ok: true,
                    message: "ok".to_string(),
                }),
                Err(err) => results.push(WslConfigureCliReport {
                    cli_key: "claude".to_string(),
                    ok: false,
                    message: err,
                }),
            }
        }

        if targets.codex {
            match configure_wsl_codex(distro, proxy_origin) {
                Ok(()) => results.push(WslConfigureCliReport {
                    cli_key: "codex".to_string(),
                    ok: true,
                    message: "ok".to_string(),
                }),
                Err(err) => results.push(WslConfigureCliReport {
                    cli_key: "codex".to_string(),
                    ok: false,
                    message: err,
                }),
            }
        }

        if targets.gemini {
            match configure_wsl_gemini(distro, proxy_origin) {
                Ok(()) => results.push(WslConfigureCliReport {
                    cli_key: "gemini".to_string(),
                    ok: true,
                    message: "ok".to_string(),
                }),
                Err(err) => results.push(WslConfigureCliReport {
                    cli_key: "gemini".to_string(),
                    ok: false,
                    message: err,
                }),
            }
        }

        let distro_ok = results.iter().all(|r| r.ok);
        success_ops += results.iter().filter(|r| r.ok).count();
        error_ops += results.iter().filter(|r| !r.ok).count();

        distro_reports.push(WslConfigureDistroReport {
            distro: distro.clone(),
            ok: distro_ok,
            results,
        });
    }

    let message = if error_ops > 0 {
        format!(
            "已配置：{success_ops} 项；失败：{error_ops} 项（可展开查看每个 distro 的详细结果）"
        )
    } else {
        format!("配置成功：{success_ops} 项")
    };

    WslConfigureReport {
        ok: success_ops > 0,
        message,
        distros: distro_reports,
    }
}

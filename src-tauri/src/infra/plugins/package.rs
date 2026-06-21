//! Usage: Safe `.aio-plugin` package extraction and manifest loading.

use crate::domain::plugins::PluginManifest;
use crate::shared::error::{AppError, AppResult};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct PluginPackageLimits {
    pub(crate) max_package_bytes: u64,
    pub(crate) max_entries: usize,
    pub(crate) max_extracted_bytes: u64,
}

impl Default for PluginPackageLimits {
    fn default() -> Self {
        Self {
            max_package_bytes: 32 * 1024 * 1024,
            max_entries: 256,
            max_extracted_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExtractedPluginPackage {
    pub(crate) root_dir: PathBuf,
    pub(crate) manifest: PluginManifest,
    pub(crate) checksum: String,
    pub(crate) package_bytes: Vec<u8>,
}

pub(crate) fn extract_plugin_package(
    package_path: &Path,
    staging_dir: &Path,
    limits: PluginPackageLimits,
) -> AppResult<ExtractedPluginPackage> {
    let metadata = std::fs::metadata(package_path).map_err(|error| {
        AppError::new(
            "PLUGIN_PACKAGE_NOT_FOUND",
            format!("failed to read plugin package metadata: {error}"),
        )
    })?;
    if metadata.len() > limits.max_package_bytes {
        return Err(AppError::new(
            "PLUGIN_PACKAGE_TOO_LARGE",
            format!(
                "plugin package exceeds {} bytes: {}",
                limits.max_package_bytes,
                package_path.display()
            ),
        ));
    }

    let bytes = std::fs::read(package_path).map_err(|error| {
        AppError::new(
            "PLUGIN_PACKAGE_READ_FAILED",
            format!("failed to read plugin package: {error}"),
        )
    })?;
    if bytes.len() as u64 > limits.max_package_bytes {
        return Err(AppError::new(
            "PLUGIN_PACKAGE_TOO_LARGE",
            format!(
                "plugin package exceeds {} bytes: {}",
                limits.max_package_bytes,
                package_path.display()
            ),
        ));
    }

    if staging_dir.exists() {
        std::fs::remove_dir_all(staging_dir).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_STAGING_FAILED",
                format!(
                    "failed to clean staging dir {}: {error}",
                    staging_dir.display()
                ),
            )
        })?;
    }
    std::fs::create_dir_all(staging_dir).map_err(|error| {
        AppError::new(
            "PLUGIN_PACKAGE_STAGING_FAILED",
            format!(
                "failed to create staging dir {}: {error}",
                staging_dir.display()
            ),
        )
    })?;

    let checksum = format!("sha256:{:x}", Sha256::digest(&bytes));
    match extract_zip_bytes(bytes, staging_dir, &limits, checksum) {
        Ok(extracted) => Ok(extracted),
        Err(error) => {
            let _ = std::fs::remove_dir_all(staging_dir);
            Err(error)
        }
    }
}

fn extract_zip_bytes(
    bytes: Vec<u8>,
    staging_dir: &Path,
    limits: &PluginPackageLimits,
    checksum: String,
) -> AppResult<ExtractedPluginPackage> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes.as_slice())).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_INVALID_ARCHIVE",
                format!("failed to open plugin package archive: {error}"),
            )
        })?;
    if archive.len() > limits.max_entries {
        return Err(AppError::new(
            "PLUGIN_PACKAGE_TOO_MANY_ENTRIES",
            format!(
                "plugin package has too many entries: {} > {}",
                archive.len(),
                limits.max_entries
            ),
        ));
    }

    let mut entries = Vec::with_capacity(archive.len());
    let mut extracted_bytes = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_INVALID_ARCHIVE",
                format!("failed to read plugin package entry: {error}"),
            )
        })?;
        let relative_path = safe_zip_entry_path(file.name())?;
        if relative_path.as_os_str().is_empty() {
            continue;
        }
        if !file.is_dir() {
            extracted_bytes = extracted_bytes.checked_add(file.size()).ok_or_else(|| {
                AppError::new(
                    "PLUGIN_PACKAGE_EXTRACTED_TOO_LARGE",
                    "plugin package extracted size overflowed",
                )
            })?;
            if extracted_bytes > limits.max_extracted_bytes {
                return Err(AppError::new(
                    "PLUGIN_PACKAGE_EXTRACTED_TOO_LARGE",
                    format!(
                        "plugin package extracted content exceeds {} bytes",
                        limits.max_extracted_bytes
                    ),
                ));
            }
        }
        entries.push((index, relative_path, file.is_dir()));
    }

    for (index, relative_path, is_dir) in entries {
        let out_path = staging_dir.join(&relative_path);
        if is_dir {
            std::fs::create_dir_all(&out_path).map_err(|error| {
                AppError::new(
                    "PLUGIN_PACKAGE_EXTRACT_FAILED",
                    format!("failed to create {}: {error}", out_path.display()),
                )
            })?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AppError::new(
                    "PLUGIN_PACKAGE_EXTRACT_FAILED",
                    format!("failed to create {}: {error}", parent.display()),
                )
            })?;
        }

        let mut file = archive.by_index(index).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_INVALID_ARCHIVE",
                format!("failed to read plugin package entry: {error}"),
            )
        })?;
        let mut out_file = std::fs::File::create(&out_path).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_EXTRACT_FAILED",
                format!("failed to create {}: {error}", out_path.display()),
            )
        })?;
        let copied = std::io::copy(&mut file, &mut out_file).map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_EXTRACT_FAILED",
                format!("failed to write {}: {error}", out_path.display()),
            )
        })?;
        if copied > limits.max_extracted_bytes {
            return Err(AppError::new(
                "PLUGIN_PACKAGE_EXTRACTED_TOO_LARGE",
                format!(
                    "plugin package extracted content exceeds {} bytes",
                    limits.max_extracted_bytes
                ),
            ));
        }
        out_file.flush().map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_EXTRACT_FAILED",
                format!("failed to flush {}: {error}", out_path.display()),
            )
        })?;
    }

    let root_dir = package_root_dir(staging_dir)?;
    let manifest_path = root_dir.join("plugin.json");
    let manifest_bytes = crate::shared::fs::read_file_with_max_len(&manifest_path, 256 * 1024)
        .map_err(|_| {
            AppError::new(
                "PLUGIN_PACKAGE_MISSING_MANIFEST",
                "plugin package must contain plugin.json",
            )
        })?;
    let manifest: PluginManifest = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        AppError::new(
            "PLUGIN_INVALID_MANIFEST",
            format!("failed to parse plugin package manifest: {error}"),
        )
    })?;
    crate::domain::plugins::validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))?;

    Ok(ExtractedPluginPackage {
        root_dir,
        manifest,
        checksum,
        package_bytes: bytes,
    })
}

fn safe_zip_entry_path(raw_name: &str) -> AppResult<PathBuf> {
    let normalized = raw_name.replace('\\', "/");
    if normalized.is_empty() {
        return Ok(PathBuf::new());
    }
    let path = Path::new(&normalized);
    if path.is_absolute() || normalized.starts_with('/') {
        return Err(AppError::new(
            "PLUGIN_PACKAGE_INVALID_PATH",
            format!("plugin package entry path must be relative: {raw_name}"),
        ));
    }

    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let segment = value.to_string_lossy();
                if segment.is_empty() || segment == "." || segment == ".." {
                    return Err(AppError::new(
                        "PLUGIN_PACKAGE_INVALID_PATH",
                        format!("plugin package entry path is invalid: {raw_name}"),
                    ));
                }
                out.push(value);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::new(
                    "PLUGIN_PACKAGE_INVALID_PATH",
                    format!("plugin package entry path is invalid: {raw_name}"),
                ));
            }
        }
    }
    Ok(out)
}

fn package_root_dir(staging_dir: &Path) -> AppResult<PathBuf> {
    let direct_manifest = staging_dir.join("plugin.json");
    if direct_manifest.exists() {
        return Ok(staging_dir.to_path_buf());
    }

    let mut dirs = Vec::new();
    let mut files = 0_usize;
    let entries = std::fs::read_dir(staging_dir).map_err(|error| {
        AppError::new(
            "PLUGIN_PACKAGE_EXTRACT_FAILED",
            format!(
                "failed to read staging dir {}: {error}",
                staging_dir.display()
            ),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::new(
                "PLUGIN_PACKAGE_EXTRACT_FAILED",
                format!("failed to read staging dir entry: {error}"),
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else {
            files += 1;
        }
    }

    if dirs.len() == 1 && files == 0 && dirs[0].join("plugin.json").exists() {
        return Ok(dirs.remove(0));
    }

    Err(AppError::new(
        "PLUGIN_PACKAGE_MISSING_MANIFEST",
        "plugin package must contain plugin.json at the package root or inside a single root directory",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Cursor, Write};

    fn manifest_json(plugin_id: &str) -> String {
        serde_json::json!({
            "id": plugin_id,
            "name": "Local Test Plugin",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "declarativeRules",
                "rules": ["rules/main.json"]
            },
            "hooks": [
                {
                    "name": "gateway.request.afterBodyRead",
                    "priority": 10,
                    "failurePolicy": "fail-open"
                }
            ],
            "permissions": ["request.body.read"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        })
        .to_string()
    }

    fn write_package(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).expect("create package");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::<()>::default();
        for (name, bytes) in entries {
            zip.start_file(*name, opts).expect("start file");
            zip.write_all(bytes).expect("write file");
        }
        zip.finish().expect("finish package");
    }

    #[test]
    fn plugin_package_security_rejects_zip_slip_entries() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("evil.aio-plugin");
        write_package(
            &package_path,
            &[
                ("plugin.json", manifest_json("local.safe").as_bytes()),
                ("../outside.txt", b"owned"),
            ],
        );

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_PACKAGE_INVALID_PATH:"));
        assert!(!dir.path().join("outside.txt").exists());
    }

    #[test]
    fn plugin_package_security_rejects_missing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("missing-manifest.aio-plugin");
        write_package(&package_path, &[("rules/main.json", b"{}")]);

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_PACKAGE_MISSING_MANIFEST:"));
    }

    #[test]
    fn plugin_package_security_rejects_package_size_over_limit() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("too-large.aio-plugin");
        write_package(
            &package_path,
            &[("plugin.json", manifest_json("local.large").as_bytes())],
        );

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits {
                max_package_bytes: 1,
                ..PluginPackageLimits::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_PACKAGE_TOO_LARGE:"));
    }

    #[test]
    fn plugin_package_security_extracts_valid_package_and_reads_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("valid.aio-plugin");
        write_package(
            &package_path,
            &[
                ("plugin.json", manifest_json("local.safe").as_bytes()),
                ("rules/main.json", br#"{"rules":[]}"#),
                ("README.md", b"# Local Test Plugin\n"),
            ],
        );

        let extracted = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap();

        assert_eq!(extracted.manifest.id, "local.safe");
        assert!(extracted.root_dir.join("rules/main.json").exists());
        assert!(extracted.checksum.starts_with("sha256:"));
    }

    #[test]
    fn plugin_package_security_accepts_single_top_level_directory() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("nested.aio-plugin");
        write_package(
            &package_path,
            &[
                (
                    "local-safe/plugin.json",
                    manifest_json("local.safe").as_bytes(),
                ),
                ("local-safe/rules/main.json", br#"{"rules":[]}"#),
            ],
        );

        let extracted = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap();

        assert_eq!(extracted.manifest.id, "local.safe");
        assert_eq!(
            extracted.root_dir.file_name().and_then(|v| v.to_str()),
            Some("local-safe")
        );
    }

    #[test]
    fn plugin_package_security_rejects_extracted_size_over_limit() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("expanded-too-large.aio-plugin");
        write_package(
            &package_path,
            &[
                ("plugin.json", manifest_json("local.safe").as_bytes()),
                ("rules/main.json", br#"{"rules":[]}"#),
            ],
        );

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits {
                max_extracted_bytes: 8,
                ..PluginPackageLimits::default()
            },
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_PACKAGE_EXTRACTED_TOO_LARGE:"));
    }

    #[test]
    fn plugin_package_security_rejects_too_many_entries() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("too-many-entries.aio-plugin");
        write_package(
            &package_path,
            &[
                ("plugin.json", manifest_json("local.safe").as_bytes()),
                ("rules/main.json", br#"{"rules":[]}"#),
            ],
        );

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits {
                max_entries: 1,
                ..PluginPackageLimits::default()
            },
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_PACKAGE_TOO_MANY_ENTRIES:"));
    }

    #[test]
    fn plugin_package_security_rejects_invalid_zip() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("invalid.aio-plugin");
        std::fs::write(&package_path, b"not a zip").unwrap();

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_PACKAGE_INVALID_ARCHIVE:"));
    }

    #[test]
    fn plugin_package_security_rejects_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let package_path = dir.path().join("absolute.aio-plugin");
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::FileOptions::<()>::default();
            zip.start_file("/tmp/plugin.json", opts)
                .expect("start file");
            zip.write_all(manifest_json("local.safe").as_bytes())
                .expect("write");
            zip.finish().expect("finish package");
        }
        std::fs::write(&package_path, buf.into_inner()).unwrap();

        let err = extract_plugin_package(
            &package_path,
            &dir.path().join("staging"),
            PluginPackageLimits::default(),
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_PACKAGE_INVALID_PATH:"));
    }
}

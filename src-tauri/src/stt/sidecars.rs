//! Sidecar (engine) binary manifests and URL resolution per OS/arch.
//! Used for on-demand download from GitHub releases; binaries live under data/sidecars/.

use serde::Serialize;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tauri::Emitter;

/// Identifies a sidecar (engine) used by a local model.
pub const SIDECAR_ID_SHERPA_ONNX: &str = "sherpa-onnx";
/// whisper-cpp: ggml-org/whisper.cpp; Windows x64/Win32 from releases.
pub const SIDECAR_ID_WHISPER_CPP: &str = "whisper-cpp";

/// Per-platform download info for a sidecar.
#[derive(Debug, Clone)]
pub struct SidecarPlatform {
    /// Full URL to the release asset (e.g. tar.bz2 or .zip).
    pub url: String,
    /// If true, asset is an archive and must be extracted.
    pub is_archive: bool,
    /// If true, archive is .zip (otherwise tar.bz2).
    pub is_zip: bool,
    /// Path to the binary inside the sidecar dir after extraction (e.g. "bin/sherpa-onnx-offline-websocket-server").
    /// For raw .exe downloads this is just the executable name.
    pub binary_relative_path: &'static str,
}

/// Returns (os, arch) for URL resolution. Normalizes to our manifest keys.
fn current_platform() -> (&'static str, &'static str) {
    let os = std::env::consts::OS; // "windows", "macos", "linux"
    let arch = std::env::consts::ARCH; // "x86_64", "aarch64", "x86", etc.
    (os, arch)
}

/// Base URL for sherpa-onnx releases (tag appended).
const SHERPA_RELEASE_TAG: &str = "v1.12.28";
const SHERPA_BASE: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download";

fn sherpa_url(path: &str) -> String {
    format!("{}/{}/{}", SHERPA_BASE, SHERPA_RELEASE_TAG, path)
}

/// Resolves platform-specific download info for sherpa-onnx.
fn sherpa_platform_url(os: &str, arch: &str) -> Option<SidecarPlatform> {
    let (url, binary_rel) = match (os, arch) {
        ("windows", "x86_64") => (
            sherpa_url("sherpa-onnx-v1.12.28-win-x64-shared-MT-Release-no-tts.tar.bz2"),
            "bin/sherpa-onnx-offline-websocket-server.exe",
        ),
        ("windows", "aarch64" | "arm64") => (
            sherpa_url("sherpa-onnx-v1.12.28-win-arm64-shared-MT-Release-no-tts.tar.bz2"),
            "bin/sherpa-onnx-offline-websocket-server.exe",
        ),
        ("linux", "x86_64") => (
            sherpa_url("sherpa-onnx-v1.12.28-linux-x64-shared-no-tts.tar.bz2"),
            "bin/sherpa-onnx-offline-websocket-server",
        ),
        ("linux", "aarch64") => (
            sherpa_url("sherpa-onnx-v1.12.28-linux-aarch64-static.tar.bz2"),
            "bin/sherpa-onnx-offline-websocket-server",
        ),
        ("macos", _) => (
            sherpa_url("sherpa-onnx-v1.12.28-onnxruntime-1.18.1-osx-universal2-shared.tar.bz2"),
            "bin/sherpa-onnx-offline-websocket-server",
        ),
        _ => return None,
    };
    Some(SidecarPlatform {
        url,
        is_archive: true,
        is_zip: false,
        binary_relative_path: binary_rel,
    })
}

/// whisper-cpp: ggml-org/whisper.cpp releases (Windows binaries; server binary name whisper-server.exe).
const WHISPER_RELEASE_TAG: &str = "v1.8.3";
const WHISPER_BASE: &str = "https://github.com/ggml-org/whisper.cpp/releases/download";

fn whisper_url(filename: &str) -> String {
    format!("{}/{}/{}", WHISPER_BASE, WHISPER_RELEASE_TAG, filename)
}

/// Resolves platform-specific download info for whisper-cpp. Official releases only provide Windows zips.
fn whisper_platform_url(os: &str, arch: &str) -> Option<SidecarPlatform> {
    let (url, binary_rel) = match (os, arch) {
        ("windows", "x86_64") => (
            whisper_url("whisper-bin-x64.zip"),
            "whisper-server.exe",
        ),
        ("windows", "x86") => (
            whisper_url("whisper-bin-Win32.zip"),
            "whisper-server.exe",
        ),
        _ => return None,
    };
    Some(SidecarPlatform {
        url,
        is_archive: true,
        is_zip: true,
        binary_relative_path: binary_rel,
    })
}

/// Returns sidecar download info for the current platform, or None if not supported.
pub fn sidecar_download_info(sidecar_id: &str) -> Option<SidecarPlatform> {
    match sidecar_id {
        SIDECAR_ID_SHERPA_ONNX => {
            let (os, arch) = current_platform();
            sherpa_platform_url(os, arch)
        }
        SIDECAR_ID_WHISPER_CPP => {
            let (os, arch) = current_platform();
            whisper_platform_url(os, arch)
        }
        _ => None,
    }
}

/// Sidecars directory under app data: .../Kalam/Kalam/data/sidecars/
pub fn sidecars_dir() -> anyhow::Result<PathBuf> {
    let dir = directories::ProjectDirs::from("com", "Kalam", "Kalam")
        .ok_or_else(|| anyhow::anyhow!("No project dir"))?
        .data_local_dir()
        .join("sidecars");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for a given sidecar: .../sidecars/<sidecar_id>/
pub fn sidecar_dir(sidecar_id: &str) -> anyhow::Result<PathBuf> {
    Ok(sidecars_dir()?.join(sidecar_id))
}

/// Path to the sidecar binary for the current platform if already installed.
/// Does not check existence; use for spawning or existence check.
pub fn sidecar_binary_path(sidecar_id: &str) -> anyhow::Result<PathBuf> {
    let info = sidecar_download_info(sidecar_id)
        .ok_or_else(|| anyhow::anyhow!("Sidecar {} not available on this platform", sidecar_id))?;
    let dir = sidecar_dir(sidecar_id)?;
    Ok(dir.join(info.binary_relative_path))
}

/// Whether the sidecar binary is installed (file exists).
pub fn sidecar_is_installed(sidecar_id: &str) -> bool {
    match sidecar_binary_path(sidecar_id) {
        Ok(p) => p.exists(),
        Err(_) => false,
    }
}

/// Map model_id to sidecar_id for local models.
pub fn model_id_to_sidecar_id(model_id: &str) -> Option<&'static str> {
    match model_id {
        "sensevoice" => Some(SIDECAR_ID_SHERPA_ONNX),
        "whisper_base" => Some(SIDECAR_ID_WHISPER_CPP),
        _ => None,
    }
}

/// Path to the sidecar binary for a model (e.g. sensevoice -> sherpa-onnx binary path).
pub fn sidecar_path_for_model(model_id: &str) -> anyhow::Result<PathBuf> {
    let sidecar_id = model_id_to_sidecar_id(model_id)
        .ok_or_else(|| anyhow::anyhow!("No sidecar for model {}", model_id))?;
    sidecar_binary_path(sidecar_id)
}

/// Whether the engine required by this model is installed.
pub fn sidecar_installed_for_model(model_id: &str) -> bool {
    model_id_to_sidecar_id(model_id)
        .map(sidecar_is_installed)
        .unwrap_or(false)
}

/// Whether the engine for this model has a download available on the current platform.
/// When false, the UI should show "Engine not available on this platform" instead of offering install/start.
pub fn sidecar_available_for_model(model_id: &str) -> bool {
    model_id_to_sidecar_id(model_id)
        .map(|id| sidecar_download_info(id).is_some())
        .unwrap_or(false)
}

// --- Download with progress ---

#[derive(Clone, Serialize)]
pub struct SidecarDownloadProgressPayload {
    pub sidecar_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f32>,
}

fn extract_tar_bz2(archive_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let dec = bzip2::read::BzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(dec);
    archive.unpack(out_dir)?;
    Ok(())
}

fn sanitize_zip_entry_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(name.replace('\\', "/"));
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => return None,
            std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
            std::path::Component::Normal(s) => out.push(s),
        }
    }
    Some(out)
}

fn extract_zip(archive_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name();
        let Some(rel_path) = sanitize_zip_entry_path(name) else { continue };
        let out_path = out_dir.join(&rel_path);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(p) = out_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

/// Download sidecar binary (with optional extraction), emit progress, set executable on Unix.
pub async fn download_sidecar_with_progress(
    sidecar_id: &str,
    app_handle: &tauri::AppHandle,
) -> anyhow::Result<PathBuf> {
    let info = sidecar_download_info(sidecar_id)
        .ok_or_else(|| anyhow::anyhow!("Sidecar {} not available on this platform", sidecar_id))?;

    let dest_dir = sidecar_dir(sidecar_id)?;
    std::fs::create_dir_all(&dest_dir)?;

    let archive_ext = if info.is_zip { "zip" } else { "tar.bz2" };
    let archive_path = dest_dir.join(format!("archive.{}", archive_ext));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let response = client.get(&info.url).send().await?;
    let total_bytes = response.content_length();
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed: {}", response.status()));
    }

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&archive_path).await?;

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let len = chunk.len() as u64;
        downloaded += len;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        let percent = total_bytes.map(|t| (downloaded as f32 / t as f32) * 100.0);
        let payload = SidecarDownloadProgressPayload {
            sidecar_id: sidecar_id.to_string(),
            downloaded_bytes: downloaded,
            total_bytes,
            percent,
        };
        let _ = app_handle.emit("sidecar-download-progress", &payload);
    }

    file.sync_all().await?;
    drop(file);

    if info.is_archive {
        let extract_tmp = dest_dir.join(".extract_tmp");
        std::fs::create_dir_all(&extract_tmp)?;
        if info.is_zip {
            extract_zip(&archive_path, &extract_tmp)?;
        } else {
            extract_tar_bz2(&archive_path, &extract_tmp)?;
        }
        let _ = std::fs::remove_file(&archive_path);

        let entries: Vec<_> = std::fs::read_dir(&extract_tmp)?
            .filter_map(Result::ok)
            .collect();
        if info.is_zip && entries.len() > 1 {
            // Zip with multiple files at root (e.g. whisper): move each into dest_dir
            for entry in entries {
                let src_path = entry.path();
                let dst = dest_dir.join(entry.file_name());
                if src_path.is_dir() {
                    copy_dir_all(&src_path, &dst)?;
                } else {
                    std::fs::copy(&src_path, &dst)?;
                }
            }
        } else {
            let top = match entries.as_slice() {
                [one] => one.path(),
                _ => return Err(anyhow::anyhow!("Archive must have exactly one root directory")),
            };
            if top.is_dir() {
                for entry in std::fs::read_dir(&top)? {
                    let entry = entry?;
                    let src_path = entry.path();
                    let dst = dest_dir.join(entry.file_name());
                    if src_path.is_dir() {
                        copy_dir_all(&src_path, &dst)?;
                    } else {
                        std::fs::copy(&src_path, &dst)?;
                    }
                }
            } else {
                std::fs::copy(&top, dest_dir.join(top.file_name().unwrap_or_default()))?;
            }
        }
        let _ = std::fs::remove_dir_all(&extract_tmp);
    }

    let binary_path = dest_dir.join(info.binary_relative_path);
    if binary_path.exists() {
        set_executable(&binary_path)?;
    }

    Ok(binary_path)
}

/// Remove the sidecar directory and all its contents. Caller should stop any running process first.
pub fn uninstall_sidecar(sidecar_id: &str) -> anyhow::Result<()> {
    let dir = sidecar_dir(sidecar_id)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
        log::info!("Uninstalled sidecar {} from {:?}", sidecar_id, dir);
    }
    Ok(())
}

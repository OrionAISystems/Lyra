use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tracing::info;

pub const RUNTIME_VERSION: &str = "0.1.0";
pub const MODEL_NAME: &str = "parakeet-tdt-0.6b-v3.q8_0.gguf";
pub const MODEL_REVISION: &str = "541d1f99c6b0c3cd0b11a95167540bb8edefd82b";
pub const MODEL_SIZE_BYTES: u64 = 713_975_456;
pub const RUNTIME_ARCHIVE_SIZE_BYTES: u64 = 106_044_768;
pub const RUNTIME_ARCHIVE_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cuda.zip";
pub const RUNTIME_CHECKSUM_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cuda.zip.sha256";
pub const MODEL_SOURCE_URL: &str = "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3";
pub const MODEL_DOWNLOAD_URL: &str = "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/resolve/541d1f99c6b0c3cd0b11a95167540bb8edefd82b/parakeet-tdt-0.6b-v3.q8_0.gguf?download=true";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupArtifact {
    pub kind: String,
    pub name: String,
    pub source: String,
    pub size_bytes: u64,
    pub destination: String,
    pub checksum_source: String,
    pub revision: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub runtime_installed: bool,
    pub runtime_version: Option<String>,
    pub runtime_variant: Option<String>,
    pub runtime_path: String,
    pub runtime_sha256: Option<String>,
    pub runtime_verified: bool,
    pub model_installed: bool,
    pub model_name: Option<String>,
    pub model_revision: Option<String>,
    pub model_path: String,
    pub model_size_bytes: Option<u64>,
    pub model_verified: bool,
    pub gpu_name: Option<String>,
    pub gpu_vram_bytes: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeManager {
    root: PathBuf,
    status: Arc<Mutex<RuntimeStatus>>,
}

impl RuntimeManager {
    pub fn new(app: &AppHandle) -> Self {
        let root = local_root(app);
        let status = Arc::new(Mutex::new(Self::inspect(&root)));
        Self { root, status }
    }

    #[cfg(test)]
    pub fn for_tests(root: PathBuf) -> Self {
        let status = Arc::new(Mutex::new(Self::inspect(&root)));
        Self { root, status }
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| Self::inspect(&self.root))
    }

    pub fn refresh(&self) -> RuntimeStatus {
        let status = Self::inspect(&self.root);
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        status
    }

    pub fn model_path(&self) -> PathBuf {
        self.root.join("models").join(MODEL_NAME)
    }

    pub fn runtime_path(&self, variant: &str) -> PathBuf {
        self.root
            .join("runtime")
            .join(RUNTIME_VERSION)
            .join(format!("windows-x86_64-{variant}"))
    }

    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        let status = self.status();
        status.runtime_installed
            && status.runtime_verified
            && status.model_installed
            && status.model_verified
    }

    pub fn setup_artifacts(&self) -> Vec<SetupArtifact> {
        vec![
            SetupArtifact {
                kind: "runtime".to_owned(),
                name: "NeMo-Speech.cpp v0.1.0 · Windows x64 CUDA".to_owned(),
                source: RUNTIME_ARCHIVE_URL.to_owned(),
                size_bytes: RUNTIME_ARCHIVE_SIZE_BYTES,
                destination: self.runtime_path("cuda").to_string_lossy().into_owned(),
                checksum_source: RUNTIME_CHECKSUM_URL.to_owned(),
                revision: Some(RUNTIME_VERSION.to_owned()),
                license: Some("Apache-2.0".to_owned()),
            },
            SetupArtifact {
                kind: "model".to_owned(),
                name: MODEL_NAME.to_owned(),
                source: MODEL_DOWNLOAD_URL.to_owned(),
                size_bytes: MODEL_SIZE_BYTES,
                destination: self.model_path().to_string_lossy().into_owned(),
                checksum_source: MODEL_SOURCE_URL.to_owned(),
                revision: Some(MODEL_REVISION.to_owned()),
                license: Some("CC BY 4.0".to_owned()),
            },
        ]
    }

    #[allow(dead_code)]
    pub fn mark_runtime_verified(&self, variant: &str, sha256: &str) -> Result<(), RuntimeError> {
        let directory = self.runtime_path(variant);
        fs::create_dir_all(&directory)?;
        let marker = RuntimeMarker {
            version: RUNTIME_VERSION.to_owned(),
            variant: variant.to_owned(),
            sha256: sha256.to_owned(),
        };
        fs::write(
            directory.join("runtime.json"),
            serde_json::to_vec_pretty(&marker)?,
        )?;
        info!(
            variant,
            version = RUNTIME_VERSION,
            "native runtime marked verified"
        );
        self.refresh();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn mark_model_verified(&self, sha256: &str) -> Result<(), RuntimeError> {
        let model_dir = self.root.join("models");
        fs::create_dir_all(&model_dir)?;
        let marker = ModelMarker {
            name: MODEL_NAME.to_owned(),
            revision: MODEL_REVISION.to_owned(),
            sha256: sha256.to_owned(),
            size_bytes: MODEL_SIZE_BYTES,
        };
        fs::write(
            model_dir.join("model.json"),
            serde_json::to_vec_pretty(&marker)?,
        )?;
        info!(
            model = MODEL_NAME,
            revision = MODEL_REVISION,
            "model marked verified"
        );
        self.refresh();
        Ok(())
    }

    fn inspect(root: &Path) -> RuntimeStatus {
        let runtime_root = root.join("runtime").join(RUNTIME_VERSION);
        let (runtime_installed, runtime_variant, runtime_sha256, runtime_verified) =
            inspect_runtime(&runtime_root);
        let model_path = root.join("models").join(MODEL_NAME);
        let model_marker_path = root.join("models").join("model.json");
        let model_marker = read_model_marker(&model_marker_path);
        let (gpu_name, gpu_vram_bytes) = detect_gpu();
        let model_installed = model_path.is_file();
        let model_verified = model_installed
            && model_marker.as_ref().is_some_and(|marker| {
                marker.name == MODEL_NAME
                    && marker.revision == MODEL_REVISION
                    && marker.size_bytes == MODEL_SIZE_BYTES
            });
        RuntimeStatus {
            runtime_installed,
            runtime_version: runtime_installed.then(|| RUNTIME_VERSION.to_owned()),
            runtime_variant,
            runtime_path: runtime_root.to_string_lossy().into_owned(),
            runtime_sha256,
            runtime_verified,
            model_installed,
            model_name: model_installed.then(|| MODEL_NAME.to_owned()),
            model_revision: model_marker.map(|marker| marker.revision),
            model_path: model_path.to_string_lossy().into_owned(),
            model_size_bytes: model_installed.then_some(MODEL_SIZE_BYTES),
            model_verified,
            gpu_name,
            gpu_vram_bytes,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RuntimeMarker {
    version: String,
    variant: String,
    sha256: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ModelMarker {
    name: String,
    revision: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("runtime/model I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("runtime/model metadata serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

fn local_root(app: &AppHandle) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("Lyra");
        }
    }
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("app-data"))
        .join("Lyra")
}

fn inspect_runtime(runtime_root: &Path) -> (bool, Option<String>, Option<String>, bool) {
    let mut found = None;
    let mut verified = None;
    let Ok(entries) = fs::read_dir(runtime_root) else {
        return (false, None, None, false);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let marker_path = path.join("runtime.json");
        let marker = fs::read_to_string(&marker_path)
            .ok()
            .and_then(|contents| serde_json::from_str::<RuntimeMarker>(&contents).ok());
        let has_dll = contains_dll(&path);
        if has_dll {
            let variant = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix("windows-x86_64-"))
                .map(ToOwned::to_owned);
            found = variant.or_else(|| Some("unknown".to_owned()));
            if let Some(marker) = marker.filter(|marker| {
                marker.version == RUNTIME_VERSION
                    && marker.variant == found.as_deref().unwrap_or_default()
            }) {
                verified = Some(marker.sha256);
            }
            break;
        }
    }
    let installed = found.is_some();
    let sha256 = verified;
    let is_verified = installed && sha256.is_some();
    (installed, found, sha256, is_verified)
}

fn contains_dll(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_dll(&path)
        } else {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        }
    })
}

fn read_model_marker(path: &Path) -> Option<ModelMarker> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

fn detect_gpu() -> (Option<String>, Option<u64>) {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output();
        let Ok(output) = output else {
            return (None, None);
        };
        if !output.status.success() {
            return (None, None);
        }
        let line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let mut values = line.split(',').map(str::trim);
        let name = values
            .next()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let vram_bytes = values
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .map(|mebibytes| mebibytes * 1024 * 1024);
        return (name, vram_bytes);
    }
    #[cfg(not(windows))]
    {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_runtime_is_not_ready() {
        let root = std::env::temp_dir().join(format!("lyra-runtime-test-{}", uuid::Uuid::new_v4()));
        let manager = RuntimeManager::for_tests(root);
        assert!(!manager.is_ready());
        assert!(!manager.status().model_installed);
    }

    #[test]
    fn paths_are_app_local_and_pinned() {
        let manager = RuntimeManager::for_tests(PathBuf::from("C:/Lyra"));
        assert!(manager.model_path().ends_with(MODEL_NAME));
        assert!(manager
            .runtime_path("cuda")
            .to_string_lossy()
            .contains("0.1.0"));
    }
}

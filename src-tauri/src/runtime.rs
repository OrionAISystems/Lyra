use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use reqwest::blocking::Client;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};
use uuid::Uuid;
use zip::ZipArchive;

pub const RUNTIME_VERSION: &str = "0.1.0";
pub const MODEL_NAME: &str = "parakeet-tdt-0.6b-v3.q8_0.gguf";
pub const MODEL_REVISION: &str = "541d1f99c6b0c3cd0b11a95167540bb8edefd82b";
pub const MODEL_SIZE_BYTES: u64 = 713_975_456;
pub const MODEL_SHA256: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
pub const RUNTIME_ARCHIVE_SIZE_BYTES: u64 = 106_044_768;
pub const RUNTIME_SHA256: &str = "ba024204e76ca2fa4eefa8787506c3c49e418147f627f60cf9206a582b60089c";
pub const RUNTIME_ARCHIVE_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cuda.zip";
pub const RUNTIME_CHECKSUM_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cuda.zip.sha256";
pub const RUNTIME_VULKAN_ARCHIVE_SIZE_BYTES: u64 = 21_967_184;
pub const RUNTIME_VULKAN_SHA256: &str =
    "b5e7b04a637da4eb25a60253e2db65774998e8dfb48c08b4db763009b82ac7ac";
pub const RUNTIME_VULKAN_ARCHIVE_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-vulkan.zip";
pub const RUNTIME_VULKAN_CHECKSUM_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-vulkan.zip.sha256";
pub const RUNTIME_CPU_ARCHIVE_SIZE_BYTES: u64 = 4_730_421;
pub const RUNTIME_CPU_SHA256: &str =
    "5e4ea81046012edcd77fd8848de8eefb5a4ba38cc26f52eb544ab184695a75d6";
pub const RUNTIME_CPU_ARCHIVE_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cpu.zip";
pub const RUNTIME_CPU_CHECKSUM_URL: &str = "https://github.com/NVIDIA/NeMo-Speech.cpp/releases/download/v0.1.0/nemo-speech-0.1.0-windows-x86_64-cpu.zip.sha256";
pub const MODEL_SOURCE_URL: &str = "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3";
pub const MODEL_DOWNLOAD_URL: &str = "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/resolve/541d1f99c6b0c3cd0b11a95167540bb8edefd82b/parakeet-tdt-0.6b-v3.q8_0.gguf?download=true";
pub const NEMOTRON_MODEL_NAME: &str = "nemotron-speech-streaming-en-0.6b.q8_0.gguf";
pub const NEMOTRON_MODEL_REVISION: &str = "ebe59e5a817142986528bbbee5dba8db7b38ed50";
pub const NEMOTRON_MODEL_SIZE_BYTES: u64 = 699_872_960;
pub const NEMOTRON_MODEL_SHA256: &str = "d9a01898d2a611c8764e23a1c2f45e70bbd5a425dc4de93692ac951dd603812d";
pub const NEMOTRON_MODEL_SOURCE_URL: &str = "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b";
pub const NEMOTRON_MODEL_DOWNLOAD_URL: &str = "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b/resolve/ebe59e5a817142986528bbbee5dba8db7b38ed50/nemotron-speech-streaming-en-0.6b.q8_0.gguf?download=true";

const SETUP_PROGRESS_EVENT: &str = "lyra://setup-progress";
const DOWNLOAD_CHUNK_BYTES: usize = 64 * 1024;
const PROGRESS_UPDATE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeVariant {
    Cuda,
    Vulkan,
    Cpu,
}

impl RuntimeVariant {
    fn parse(value: &str) -> Result<Self, RuntimeError> {
        match value {
            "cuda" => Ok(Self::Cuda),
            "vulkan" => Ok(Self::Vulkan),
            "cpu" => Ok(Self::Cpu),
            other => Err(RuntimeError::InvalidRuntimeVariant(other.to_owned())),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Cuda => "cuda",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupArtifactKind {
    Runtime(RuntimeVariant),
    ModelParakeet,
    ModelNemotron,
}

impl SetupArtifactKind {
    pub fn parse(value: &str) -> Result<Self, RuntimeError> {
        match value {
            "runtime" | "runtime-cuda" => Ok(Self::Runtime(RuntimeVariant::Cuda)),
            "runtime-vulkan" => Ok(Self::Runtime(RuntimeVariant::Vulkan)),
            "runtime-cpu" => Ok(Self::Runtime(RuntimeVariant::Cpu)),
            "model" | "model-parakeet" => Ok(Self::ModelParakeet),
            "model-nemotron" => Ok(Self::ModelNemotron),
            other => Err(RuntimeError::InvalidArtifact(other.to_owned())),
        }
    }

    pub fn parse_with_variant(value: &str, variant: Option<&str>) -> Result<Self, RuntimeError> {
        if value != "runtime" {
            return Self::parse(value);
        }
        Ok(Self::Runtime(RuntimeVariant::parse(
            variant.unwrap_or("cuda"),
        )?))
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Runtime(_) => "runtime",
            Self::ModelParakeet | Self::ModelNemotron => "model",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Runtime(variant) => match variant {
                RuntimeVariant::Cuda => "runtime-cuda",
                RuntimeVariant::Vulkan => "runtime-vulkan",
                RuntimeVariant::Cpu => "runtime-cpu",
            },
            Self::ModelParakeet => "model-parakeet",
            Self::ModelNemotron => "model-nemotron",
        }
    }

    fn runtime_variant(self) -> Option<RuntimeVariant> {
        match self {
            Self::Runtime(variant) => Some(variant),
            Self::ModelParakeet | Self::ModelNemotron => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ArtifactSpec {
    kind: SetupArtifactKind,
    name: &'static str,
    source: &'static str,
    size_bytes: u64,
    checksum_source: &'static str,
    expected_sha256: &'static str,
    destination: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupArtifact {
    pub kind: String,
    pub variant: Option<String>,
    pub name: String,
    pub source: String,
    pub size_bytes: u64,
    pub destination: String,
    pub checksum_source: String,
    pub expected_sha256: String,
    pub revision: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupProgress {
    pub kind: Option<String>,
    pub variant: Option<String>,
    pub state: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

impl Default for SetupProgress {
    fn default() -> Self {
        Self {
            kind: None,
            variant: None,
            state: "idle".to_owned(),
            downloaded_bytes: 0,
            total_bytes: 0,
            error: None,
        }
    }
}

#[derive(Debug)]
struct SetupControl {
    progress: SetupProgress,
    cancellation: Option<Arc<AtomicBool>>,
}

impl Default for SetupControl {
    fn default() -> Self {
        Self {
            progress: SetupProgress::default(),
            cancellation: None,
        }
    }
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
    pub nemotron_model_installed: bool,
    pub nemotron_model_verified: bool,
    pub nemotron_model_path: String,
    pub gpu_name: Option<String>,
    pub gpu_vram_bytes: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeManager {
    root: PathBuf,
    status: Arc<Mutex<RuntimeStatus>>,
    setup: Arc<Mutex<SetupControl>>,
}

impl RuntimeManager {
    pub fn new(app: &AppHandle) -> Self {
        let root = local_root(app);
        let status = Arc::new(Mutex::new(Self::inspect(&root)));
        Self {
            root,
            status,
            setup: Arc::new(Mutex::new(SetupControl::default())),
        }
    }

    #[cfg(test)]
    pub fn for_tests(root: PathBuf) -> Self {
        let status = Arc::new(Mutex::new(Self::inspect(&root)));
        Self {
            root,
            status,
            setup: Arc::new(Mutex::new(SetupControl::default())),
        }
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| Self::inspect(&self.root))
    }

    pub fn refresh(&self) -> RuntimeStatus {
        let mut status = Self::inspect(&self.root);
        if let Ok(mut current) = self.status.lock() {
            status.last_error = current.last_error.clone();
            *current = status.clone();
        }
        status
    }

    pub fn setup_progress(&self) -> SetupProgress {
        self.setup
            .lock()
            .map(|setup| setup.progress.clone())
            .unwrap_or_default()
    }

    pub fn model_path(&self) -> PathBuf {
        self.root.join("models").join(MODEL_NAME)
    }

    pub fn nemotron_model_path(&self) -> PathBuf {
        self.root.join("models").join(NEMOTRON_MODEL_NAME)
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

    pub fn is_variant_ready(&self, variant: &str) -> bool {
        let runtime_root = self.root.join("runtime").join(RUNTIME_VERSION);
        inspect_runtime_variant(&runtime_root, variant).is_some_and(|(_, _, _, verified)| verified)
    }

    pub fn verified_fallbacks(&self, active_variant: &str) -> Vec<String> {
        ["vulkan", "cpu"]
            .into_iter()
            .filter(|variant| *variant != active_variant)
            .filter(|variant| self.is_variant_ready(variant))
            .map(ToOwned::to_owned)
            .collect()
    }

    pub fn setup_artifacts(&self) -> Vec<SetupArtifact> {
        let runtime_cuda = self.artifact_spec(SetupArtifactKind::Runtime(RuntimeVariant::Cuda));
        let runtime_vulkan = self.artifact_spec(SetupArtifactKind::Runtime(RuntimeVariant::Vulkan));
        let runtime_cpu = self.artifact_spec(SetupArtifactKind::Runtime(RuntimeVariant::Cpu));
        let model = self.artifact_spec(SetupArtifactKind::ModelParakeet);
        let nemotron = self.artifact_spec(SetupArtifactKind::ModelNemotron);
        vec![
            setup_artifact(&runtime_cuda),
            setup_artifact(&runtime_vulkan),
            setup_artifact(&runtime_cpu),
            setup_artifact(&model),
            setup_artifact(&nemotron),
        ]
    }

    pub fn start_setup(
        &self,
        app: &AppHandle,
        kind: SetupArtifactKind,
    ) -> Result<(), RuntimeError> {
        let cancellation = Arc::new(AtomicBool::new(false));
        {
            let mut setup = self.setup.lock().map_err(|_| RuntimeError::StatePoisoned)?;
            if matches!(
                setup.progress.state.as_str(),
                "downloading" | "verifying" | "installing"
            ) {
                return Err(RuntimeError::Busy);
            }
            setup.progress = SetupProgress {
                kind: Some(kind.as_str().to_owned()),
                variant: kind
                    .runtime_variant()
                    .map(|variant| variant.as_str().to_owned()),
                state: "downloading".to_owned(),
                downloaded_bytes: 0,
                total_bytes: self.artifact_spec(kind).size_bytes,
                error: None,
            };
            setup.cancellation = Some(cancellation.clone());
        }
        if let Ok(mut status) = self.status.lock() {
            status.last_error = None;
        }
        self.emit_setup_progress(app);

        let manager = self.clone();
        let progress_app = app.clone();
        let completion_app = app.clone();
        let worker = thread::Builder::new()
            .name(format!("lyra-setup-{}", kind.id()))
            .spawn(move || {
                let result = manager.install_artifact(kind, cancellation, |progress| {
                    let _ = progress_app.emit(SETUP_PROGRESS_EVENT, progress);
                });
                manager.finish_setup(kind, result);
                let _ = completion_app.emit(SETUP_PROGRESS_EVENT, manager.setup_progress());
                let _ = completion_app.emit("lyra://runtime-status", manager.refresh());
            });
        if let Err(error) = worker {
            let message = error.to_string();
            self.finish_setup(kind, Err(RuntimeError::Thread(message.clone())));
            return Err(RuntimeError::Thread(message));
        }
        Ok(())
    }

    pub fn cancel_setup(&self) -> bool {
        self.setup
            .lock()
            .ok()
            .and_then(|setup| setup.cancellation.clone())
            .map(|cancellation| {
                cancellation.store(true, Ordering::Release);
                true
            })
            .unwrap_or(false)
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
        write_json_atomic(&directory.join("runtime.json"), &marker)?;
        write_json_atomic(
            &directory
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("active.json"),
            &ActiveRuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: variant.to_owned(),
            },
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
        write_json_atomic(&model_dir.join("model.json"), &marker)?;
        info!(
            model = MODEL_NAME,
            revision = MODEL_REVISION,
            "model marked verified"
        );
        self.refresh();
        Ok(())
    }

    fn artifact_spec(&self, kind: SetupArtifactKind) -> ArtifactSpec {
        match kind {
            SetupArtifactKind::Runtime(RuntimeVariant::Cuda) => ArtifactSpec {
                kind,
                name: "NeMo-Speech.cpp v0.1.0 · Windows x64 CUDA",
                source: RUNTIME_ARCHIVE_URL,
                size_bytes: RUNTIME_ARCHIVE_SIZE_BYTES,
                checksum_source: RUNTIME_CHECKSUM_URL,
                expected_sha256: RUNTIME_SHA256,
                destination: self.runtime_path("cuda"),
            },
            SetupArtifactKind::Runtime(RuntimeVariant::Vulkan) => ArtifactSpec {
                kind,
                name: "NeMo-Speech.cpp v0.1.0 · Windows x64 Vulkan",
                source: RUNTIME_VULKAN_ARCHIVE_URL,
                size_bytes: RUNTIME_VULKAN_ARCHIVE_SIZE_BYTES,
                checksum_source: RUNTIME_VULKAN_CHECKSUM_URL,
                expected_sha256: RUNTIME_VULKAN_SHA256,
                destination: self.runtime_path("vulkan"),
            },
            SetupArtifactKind::Runtime(RuntimeVariant::Cpu) => ArtifactSpec {
                kind,
                name: "NeMo-Speech.cpp v0.1.0 · Windows x64 CPU",
                source: RUNTIME_CPU_ARCHIVE_URL,
                size_bytes: RUNTIME_CPU_ARCHIVE_SIZE_BYTES,
                checksum_source: RUNTIME_CPU_CHECKSUM_URL,
                expected_sha256: RUNTIME_CPU_SHA256,
                destination: self.runtime_path("cpu"),
            },
            SetupArtifactKind::ModelParakeet => ArtifactSpec {
                kind,
                name: MODEL_NAME,
                source: MODEL_DOWNLOAD_URL,
                size_bytes: MODEL_SIZE_BYTES,
                checksum_source: MODEL_SOURCE_URL,
                expected_sha256: MODEL_SHA256,
                destination: self.model_path(),
            },
            SetupArtifactKind::ModelNemotron => ArtifactSpec {
                kind,
                name: NEMOTRON_MODEL_NAME,
                source: NEMOTRON_MODEL_DOWNLOAD_URL,
                size_bytes: NEMOTRON_MODEL_SIZE_BYTES,
                checksum_source: NEMOTRON_MODEL_SOURCE_URL,
                expected_sha256: NEMOTRON_MODEL_SHA256,
                destination: self.nemotron_model_path(),
            },
        }
    }

    fn install_artifact<F>(
        &self,
        kind: SetupArtifactKind,
        cancellation: Arc<AtomicBool>,
        mut emit_progress: F,
    ) -> Result<(), RuntimeError>
    where
        F: FnMut(SetupProgress),
    {
        let spec = self.artifact_spec(kind);
        let partial_path = self.partial_path(kind);
        self.download_to_partial(&spec, &partial_path, &cancellation, &mut emit_progress)?;

        self.set_setup_phase(kind, "verifying", 0, spec.size_bytes, None);
        emit_progress(self.setup_progress());
        if let Err(error) = verify_file(&partial_path, spec.expected_sha256, spec.size_bytes) {
            if matches!(error, RuntimeError::ChecksumMismatch { .. }) {
                let _ = fs::remove_file(&partial_path);
            }
            return Err(error);
        }

        if cancellation.load(Ordering::Acquire) {
            return Err(RuntimeError::Cancelled);
        }

        self.set_setup_phase(kind, "installing", spec.size_bytes, spec.size_bytes, None);
        emit_progress(self.setup_progress());
        match kind {
            SetupArtifactKind::Runtime(_) => self.install_runtime_archive(&partial_path, &spec)?,
            SetupArtifactKind::ModelParakeet | SetupArtifactKind::ModelNemotron => self.install_model_file(&partial_path, &spec)?,
        }
        Ok(())
    }

    fn download_to_partial<F>(
        &self,
        spec: &ArtifactSpec,
        partial_path: &Path,
        cancellation: &AtomicBool,
        emit_progress: &mut F,
    ) -> Result<(), RuntimeError>
    where
        F: FnMut(SetupProgress),
    {
        if let Some(parent) = partial_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut existing = fs::metadata(partial_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if existing > spec.size_bytes {
            fs::remove_file(partial_path)?;
            existing = 0;
        }

        if existing == spec.size_bytes {
            self.set_setup_progress(spec.kind, existing, spec.size_bytes, None);
            emit_progress(self.setup_progress());
            return Ok(());
        }

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(900))
            .build()?;
        let mut request = client.get(spec.source);
        if existing > 0 {
            request = request.header("Range", format!("bytes={existing}-"));
        }
        let mut response = request.send()?;

        if existing > 0 && response.status().as_u16() == 416 {
            fs::remove_file(partial_path)?;
            existing = 0;
            response = client.get(spec.source).send()?;
        }

        let append = existing > 0 && response.status().as_u16() == 206;
        if !response.status().is_success() {
            return Err(RuntimeError::HttpStatus {
                status: response.status().as_u16(),
                artifact: spec.kind.as_str().to_owned(),
            });
        }
        if !append {
            existing = 0;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(partial_path)?;
        let mut downloaded = existing;
        let total = spec.size_bytes;
        let mut last_reported = downloaded;
        let mut buffer = vec![0_u8; DOWNLOAD_CHUNK_BYTES];
        self.set_setup_progress(spec.kind, downloaded, total, None);
        emit_progress(self.setup_progress());

        loop {
            if cancellation.load(Ordering::Acquire) {
                return Err(RuntimeError::Cancelled);
            }
            let read = response.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            downloaded += read as u64;
            if downloaded.saturating_sub(last_reported) >= PROGRESS_UPDATE_BYTES {
                last_reported = downloaded;
                self.set_setup_progress(spec.kind, downloaded, total, None);
                emit_progress(self.setup_progress());
            }
        }
        file.flush()?;
        self.set_setup_progress(spec.kind, downloaded, total, None);
        emit_progress(self.setup_progress());

        if downloaded != spec.size_bytes {
            return Err(RuntimeError::SizeMismatch {
                artifact: spec.kind.as_str().to_owned(),
                expected: spec.size_bytes,
                actual: downloaded,
            });
        }
        Ok(())
    }

    fn install_runtime_archive(
        &self,
        partial_path: &Path,
        spec: &ArtifactSpec,
    ) -> Result<(), RuntimeError> {
        let extraction_path = self
            .root
            .join("staging")
            .join(format!("runtime-extract-{}", Uuid::new_v4()));
        if extraction_path.exists() {
            fs::remove_dir_all(&extraction_path)?;
        }
        fs::create_dir_all(&extraction_path)?;

        let extraction_result = (|| {
            let file = File::open(partial_path)?;
            let mut archive = ZipArchive::new(file)?;
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let relative = entry
                    .enclosed_name()
                    .ok_or_else(|| RuntimeError::UnsafeArchiveEntry(entry.name().to_owned()))?;
                let destination = extraction_path.join(relative);
                if entry.is_dir() {
                    fs::create_dir_all(&destination)?;
                    continue;
                }
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut output = File::create(destination)?;
                io::copy(&mut entry, &mut output)?;
            }
            if !contains_dll(&extraction_path) {
                return Err(RuntimeError::MissingRuntimeBinary);
            }
            Ok::<(), RuntimeError>(())
        })();

        if let Err(error) = extraction_result {
            let _ = fs::remove_dir_all(&extraction_path);
            return Err(error);
        }

        if let Some(parent) = spec.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if spec.destination.exists() {
            fs::remove_dir_all(&spec.destination)?;
        }
        fs::rename(&extraction_path, &spec.destination)?;
        write_json_atomic(
            &spec.destination.join("runtime.json"),
            &RuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: spec
                    .kind
                    .runtime_variant()
                    .map(RuntimeVariant::as_str)
                    .unwrap_or("cuda")
                    .to_owned(),
                sha256: spec.expected_sha256.to_owned(),
            },
        )?;
        write_json_atomic(
            &spec
                .destination
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("active.json"),
            &ActiveRuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: spec
                    .kind
                    .runtime_variant()
                    .map(RuntimeVariant::as_str)
                    .unwrap_or("cuda")
                    .to_owned(),
            },
        )?;
        fs::remove_file(partial_path)?;
        info!(path = %spec.destination.display(), "native runtime installed");
        Ok(())
    }

    fn install_model_file(
        &self,
        partial_path: &Path,
        spec: &ArtifactSpec,
    ) -> Result<(), RuntimeError> {
        let (name, revision, sha256, size_bytes, marker_name) = match spec.kind {
            SetupArtifactKind::ModelParakeet => (MODEL_NAME, MODEL_REVISION, MODEL_SHA256, MODEL_SIZE_BYTES, "model.json"),
            SetupArtifactKind::ModelNemotron => (NEMOTRON_MODEL_NAME, NEMOTRON_MODEL_REVISION, NEMOTRON_MODEL_SHA256, NEMOTRON_MODEL_SIZE_BYTES, "nemotron-model.json"),
            SetupArtifactKind::Runtime(_) => return Err(RuntimeError::InvalidArtifact("runtime passed to model installer".to_owned())),
        };
        if let Some(parent) = spec.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let installing_path = spec
            .destination
            .with_file_name(format!("{name}.installing"));
        if installing_path.exists() {
            fs::remove_file(&installing_path)?;
        }
        fs::rename(partial_path, &installing_path)?;
        if spec.destination.exists() {
            fs::remove_file(&spec.destination)?;
        }
        fs::rename(&installing_path, &spec.destination)?;
        write_json_atomic(
            &spec
                .destination
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(marker_name),
            &ModelMarker {
                name: name.to_owned(),
                revision: revision.to_owned(),
                sha256: sha256.to_owned(),
                size_bytes,
            },
        )?;
        info!(path = %spec.destination.display(), "speech model installed");
        Ok(())
    }

    fn partial_path(&self, kind: SetupArtifactKind) -> PathBuf {
        self.root
            .join("staging")
            .join(format!("{}-{}.partial", kind.id(), RUNTIME_VERSION))
    }

    fn set_setup_phase(
        &self,
        kind: SetupArtifactKind,
        state: &str,
        downloaded_bytes: u64,
        total_bytes: u64,
        error: Option<String>,
    ) {
        if let Ok(mut setup) = self.setup.lock() {
            setup.progress = SetupProgress {
                kind: Some(kind.as_str().to_owned()),
                variant: kind
                    .runtime_variant()
                    .map(|variant| variant.as_str().to_owned()),
                state: state.to_owned(),
                downloaded_bytes,
                total_bytes,
                error,
            };
        }
    }

    fn set_setup_progress(
        &self,
        kind: SetupArtifactKind,
        downloaded_bytes: u64,
        total_bytes: u64,
        error: Option<String>,
    ) {
        if let Ok(mut setup) = self.setup.lock() {
            setup.progress.kind = Some(kind.as_str().to_owned());
            setup.progress.variant = kind
                .runtime_variant()
                .map(|variant| variant.as_str().to_owned());
            setup.progress.state = "downloading".to_owned();
            setup.progress.downloaded_bytes = downloaded_bytes;
            setup.progress.total_bytes = total_bytes;
            setup.progress.error = error;
        }
    }

    fn finish_setup(&self, kind: SetupArtifactKind, result: Result<(), RuntimeError>) {
        let (state, error) = match result {
            Ok(()) => ("complete".to_owned(), None),
            Err(RuntimeError::Cancelled) => ("cancelled".to_owned(), None),
            Err(error) => {
                warn!(artifact = kind.as_str(), error = %error, "setup artifact installation failed");
                ("error".to_owned(), Some(error.to_string()))
            }
        };
        if let Ok(mut setup) = self.setup.lock() {
            setup.progress.kind = Some(kind.as_str().to_owned());
            setup.progress.variant = kind
                .runtime_variant()
                .map(|variant| variant.as_str().to_owned());
            setup.progress.state = state;
            setup.progress.error = error.clone();
            setup.cancellation = None;
        }
        if let Ok(mut status) = self.status.lock() {
            status.last_error = error;
        }
    }

    fn emit_setup_progress(&self, app: &AppHandle) {
        let _ = app.emit(SETUP_PROGRESS_EVENT, self.setup_progress());
    }

    fn inspect(root: &Path) -> RuntimeStatus {
        let runtime_root = root.join("runtime").join(RUNTIME_VERSION);
        let (runtime_installed, runtime_variant, runtime_sha256, runtime_verified) =
            inspect_runtime(&runtime_root);
        let model_path = root.join("models").join(MODEL_NAME);
        let model_marker_path = root.join("models").join("model.json");
        let model_marker = read_model_marker(&model_marker_path);
        let nemotron_model_path = root.join("models").join(NEMOTRON_MODEL_NAME);
        let nemotron_marker = read_model_marker(&root.join("models").join("nemotron-model.json"));
        let (gpu_name, gpu_vram_bytes) = detect_gpu();
        let model_installed = model_path.is_file();
        let model_verified = model_installed
            && model_marker.as_ref().is_some_and(|marker| {
                marker.name == MODEL_NAME
                    && marker.revision == MODEL_REVISION
                    && marker.sha256 == MODEL_SHA256
                    && marker.size_bytes == MODEL_SIZE_BYTES
            });
        let nemotron_model_installed = nemotron_model_path.is_file();
        let nemotron_model_verified = nemotron_model_installed && nemotron_marker.as_ref().is_some_and(|marker| marker.name == NEMOTRON_MODEL_NAME && marker.revision == NEMOTRON_MODEL_REVISION && marker.sha256 == NEMOTRON_MODEL_SHA256 && marker.size_bytes == NEMOTRON_MODEL_SIZE_BYTES);
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
            nemotron_model_installed,
            nemotron_model_verified,
            nemotron_model_path: nemotron_model_path.to_string_lossy().into_owned(),
            gpu_name,
            gpu_vram_bytes,
            last_error: None,
        }
    }
}

fn setup_artifact(spec: &ArtifactSpec) -> SetupArtifact {
    SetupArtifact {
        kind: spec.kind.as_str().to_owned(),
        variant: spec
            .kind
            .runtime_variant()
            .map(|variant| variant.as_str().to_owned()),
        name: spec.name.to_owned(),
        source: spec.source.to_owned(),
        size_bytes: spec.size_bytes,
        destination: spec.destination.to_string_lossy().into_owned(),
        checksum_source: spec.checksum_source.to_owned(),
        expected_sha256: spec.expected_sha256.to_owned(),
        revision: Some(match spec.kind { SetupArtifactKind::ModelParakeet => MODEL_REVISION, SetupArtifactKind::ModelNemotron => NEMOTRON_MODEL_REVISION, SetupArtifactKind::Runtime(_) => RUNTIME_VERSION }.to_owned()),
        license: Some(match spec.kind { SetupArtifactKind::ModelParakeet => "CC BY 4.0", SetupArtifactKind::ModelNemotron => "NVIDIA Open Model License", SetupArtifactKind::Runtime(_) => "Apache-2.0" }.to_owned()),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RuntimeMarker {
    version: String,
    variant: String,
    sha256: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ActiveRuntimeMarker {
    version: String,
    variant: String,
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
    #[error("runtime/model download failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("runtime archive extraction failed: {0}")]
    Archive(#[from] zip::result::ZipError),
    #[error("unknown setup artifact: {0}")]
    InvalidArtifact(String),
    #[error("unknown runtime variant: {0}")]
    InvalidRuntimeVariant(String),
    #[error("another setup artifact is already being installed")]
    Busy,
    #[error("setup installation was cancelled")]
    Cancelled,
    #[error("setup state is unavailable")]
    StatePoisoned,
    #[error("could not start setup worker: {0}")]
    Thread(String),
    #[error("HTTP status {status} while downloading {artifact}")]
    HttpStatus { status: u16, artifact: String },
    #[error("download size mismatch for {artifact}: expected {expected} bytes, received {actual}")]
    SizeMismatch {
        artifact: String,
        expected: u64,
        actual: u64,
    },
    #[error("checksum mismatch for {artifact}: expected {expected}, received {actual}")]
    ChecksumMismatch {
        artifact: String,
        expected: String,
        actual: String,
    },
    #[error("unsafe path in runtime archive: {0}")]
    UnsafeArchiveEntry(String),
    #[error("runtime archive does not contain a DLL")]
    MissingRuntimeBinary,
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
    let active_variant = fs::read_to_string(runtime_root.join("active.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<ActiveRuntimeMarker>(&contents).ok())
        .filter(|marker| marker.version == RUNTIME_VERSION)
        .map(|marker| marker.variant);
    if let Some(variant) = active_variant {
        if let Some(status) = inspect_runtime_variant(runtime_root, &variant) {
            return status;
        }
    }

    for variant in ["cuda", "vulkan", "cpu"] {
        if let Some(status) = inspect_runtime_variant(runtime_root, variant) {
            return status;
        }
    }

    let Ok(entries) = fs::read_dir(runtime_root) else {
        return (false, None, None, false);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(variant) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix("windows-x86_64-"))
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        if let Some(status) = inspect_runtime_variant(runtime_root, &variant) {
            return status;
        }
    }
    (false, None, None, false)
}

fn inspect_runtime_variant(
    runtime_root: &Path,
    variant: &str,
) -> Option<(bool, Option<String>, Option<String>, bool)> {
    let path = runtime_root.join(format!("windows-x86_64-{variant}"));
    if !path.is_dir() || !contains_dll(&path) {
        return None;
    }
    let marker = fs::read_to_string(path.join("runtime.json"))
        .ok()
        .and_then(|contents| serde_json::from_str::<RuntimeMarker>(&contents).ok());
    let marker_sha256 = marker.as_ref().map(|marker| marker.sha256.clone());
    let verified = marker.as_ref().is_some_and(|marker| {
        marker.version == RUNTIME_VERSION
            && marker.variant == variant
            && marker.sha256 == runtime_sha256_for_variant(variant)
    });
    Some((true, Some(variant.to_owned()), marker_sha256, verified))
}

fn runtime_sha256_for_variant(variant: &str) -> &'static str {
    match variant {
        "vulkan" => RUNTIME_VULKAN_SHA256,
        "cpu" => RUNTIME_CPU_SHA256,
        _ => RUNTIME_SHA256,
    }
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

fn verify_file(path: &Path, expected_sha256: &str, expected_size: u64) -> Result<(), RuntimeError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() != expected_size {
        return Err(RuntimeError::SizeMismatch {
            artifact: path.to_string_lossy().into_owned(),
            expected: expected_size,
            actual: metadata.len(),
        });
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; DOWNLOAD_CHUNK_BYTES];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != expected_sha256 {
        return Err(RuntimeError::ChecksumMismatch {
            artifact: path.to_string_lossy().into_owned(),
            expected: expected_sha256.to_owned(),
            actual,
        });
    }
    Ok(())
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
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
        let root = std::env::temp_dir().join(format!("lyra-runtime-test-{}", Uuid::new_v4()));
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

    #[test]
    fn setup_artifacts_expose_pinned_checksums() {
        let manager = RuntimeManager::for_tests(PathBuf::from("C:/Lyra"));
        let artifacts = manager.setup_artifacts();
        assert_eq!(artifacts.len(), 4);
        assert_eq!(artifacts[0].expected_sha256, RUNTIME_SHA256);
        assert_eq!(artifacts[1].expected_sha256, RUNTIME_VULKAN_SHA256);
        assert_eq!(artifacts[2].expected_sha256, RUNTIME_CPU_SHA256);
        assert_eq!(artifacts[3].expected_sha256, MODEL_SHA256);
        assert_eq!(artifacts[0].variant.as_deref(), Some("cuda"));
        assert_eq!(artifacts[3].variant, None);
    }

    #[test]
    fn partial_downloads_are_scoped_to_staging() {
        let manager = RuntimeManager::for_tests(PathBuf::from("C:/Lyra"));
        assert!(manager
            .partial_path(SetupArtifactKind::Runtime(RuntimeVariant::Cuda))
            .ends_with("runtime-cuda-0.1.0.partial"));
    }

    #[test]
    fn active_runtime_marker_keeps_a_user_selected_fallback() {
        let root = std::env::temp_dir().join(format!("lyra-runtime-selection-{}", Uuid::new_v4()));
        let manager = RuntimeManager::for_tests(root.clone());
        let cuda = manager.runtime_path("cuda");
        let vulkan = manager.runtime_path("vulkan");
        fs::create_dir_all(&cuda).expect("create CUDA fixture");
        fs::create_dir_all(&vulkan).expect("create Vulkan fixture");
        fs::write(cuda.join("nemo_speech_asr_c.dll"), b"cuda").expect("write CUDA fixture");
        fs::write(vulkan.join("nemo_speech_asr_c.dll"), b"vulkan").expect("write Vulkan fixture");
        write_json_atomic(
            &cuda.join("runtime.json"),
            &RuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: "cuda".to_owned(),
                sha256: RUNTIME_SHA256.to_owned(),
            },
        )
        .expect("write CUDA marker");
        write_json_atomic(
            &vulkan.join("runtime.json"),
            &RuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: "vulkan".to_owned(),
                sha256: RUNTIME_VULKAN_SHA256.to_owned(),
            },
        )
        .expect("write Vulkan marker");
        write_json_atomic(
            &cuda.parent().unwrap().join("active.json"),
            &ActiveRuntimeMarker {
                version: RUNTIME_VERSION.to_owned(),
                variant: "vulkan".to_owned(),
            },
        )
        .expect("write active marker");

        let status = manager.refresh();
        assert_eq!(status.runtime_variant.as_deref(), Some("vulkan"));
        assert!(status.runtime_verified);
        assert!(manager.is_variant_ready("cuda"));
        assert!(manager.is_variant_ready("vulkan"));
        assert!(!manager.is_variant_ready("cpu"));
        assert_eq!(manager.verified_fallbacks("cuda"), vec!["vulkan"]);
        fs::remove_dir_all(root).expect("remove runtime fixture");
    }
}

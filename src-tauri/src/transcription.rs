use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use reqwest::blocking::{multipart, Client};
use serde::Deserialize;
use tracing::{info, warn};

use crate::{
    audio::encode_pcm_wav,
    domain::{
        BackendError, LanguagePreference, TranscriptionRequest, TranscriptionResult,
        TranscriptionRoute,
    },
    native_nemo::NativeNemoRecognizer,
    runtime::RuntimeManager,
};

pub const OPENAI_TRANSCRIPTION_MODEL: &str = "gpt-4o-mini-transcribe";

pub trait TranscriptionBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionResult, BackendError>;
}

pub struct LocalNemoBackend {
    runtime: RuntimeManager,
    recognizer: Mutex<Option<NativeNemoRecognizer>>,
}

impl LocalNemoBackend {
    pub fn new(runtime: RuntimeManager) -> Self {
        Self {
            runtime,
            recognizer: Mutex::new(None),
        }
    }

    fn recognizer(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<NativeNemoRecognizer>>, BackendError> {
        let status = self.runtime.status();
        if !status.runtime_installed
            || !status.runtime_verified
            || !status.model_installed
            || !status.model_verified
        {
            return Err(BackendError::MissingRuntimeModel(
                "install and verify the local runtime and Parakeet model in setup".to_owned(),
            ));
        }
        let mut recognizer = self
            .recognizer
            .lock()
            .map_err(|_| BackendError::Unsupported("local recognizer lock poisoned".to_owned()))?;
        if recognizer.is_none() {
            let variant = status.runtime_variant.as_deref().unwrap_or("cuda");
            let gpu_index = if variant == "cpu" { -1 } else { 0 };
            let loaded = NativeNemoRecognizer::load(
                &self.runtime.runtime_path(variant),
                &self.runtime.model_path(),
                gpu_index,
            )?;
            let runtime_version = loaded.version().unwrap_or_else(|| "unknown".to_owned());
            info!(
                backend = self.name(),
                variant,
                model = crate::runtime::MODEL_NAME,
                runtime_version,
                "local recognizer warmed"
            );
            *recognizer = Some(loaded);
        }
        Ok(recognizer)
    }
}

impl TranscriptionBackend for LocalNemoBackend {
    fn name(&self) -> &'static str {
        "local_nemo"
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionResult, BackendError> {
        let started = Instant::now();
        let recognizer = self.recognizer()?;
        let text = recognizer
            .as_ref()
            .ok_or_else(|| {
                BackendError::MissingRuntimeModel("local recognizer did not initialize".to_owned())
            })?
            .recognize(request)?;
        let transcription_duration_ms = started.elapsed().as_millis() as u64;
        Ok(TranscriptionResult {
            text,
            backend: self.name().to_owned(),
            model: crate::runtime::MODEL_NAME.to_owned(),
            language: None,
            audio_duration_ms: request.duration_ms(),
            transcription_duration_ms,
            latency_ms: transcription_duration_ms,
            fallback_reason: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    const SERVICE: &'static str = "Lyra";
    const USER: &'static str = "openai-transcription";

    pub fn set_openai_key(&self, secret: &str) -> Result<(), BackendError> {
        if secret.trim().is_empty() {
            return Err(BackendError::CredentialFailure(
                "credential cannot be empty".to_owned(),
            ));
        }
        let entry = keyring::Entry::new(Self::SERVICE, Self::USER)
            .map_err(|error| BackendError::CredentialFailure(error.to_string()))?;
        entry
            .set_password(secret.trim())
            .map_err(|error| BackendError::CredentialFailure(error.to_string()))
    }

    pub fn clear_openai_key(&self) -> Result<(), BackendError> {
        let entry = keyring::Entry::new(Self::SERVICE, Self::USER)
            .map_err(|error| BackendError::CredentialFailure(error.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(error) if error.to_string().to_ascii_lowercase().contains("no entry") => Ok(()),
            Err(error) => Err(BackendError::CredentialFailure(error.to_string())),
        }
    }

    fn openai_key(&self) -> Result<String, BackendError> {
        let entry = keyring::Entry::new(Self::SERVICE, Self::USER)
            .map_err(|error| BackendError::CredentialFailure(error.to_string()))?;
        entry.get_password().map_err(|error| {
            BackendError::CredentialFailure(
                "OpenAI credential is not configured"
                    .to_owned()
                    .trim_end_matches('.')
                    .to_owned()
                    + &format!(" ({error})"),
            )
        })
    }

    pub fn has_openai_key(&self) -> bool {
        self.openai_key().is_ok()
    }
}

pub struct OpenAiTranscriptionBackend {
    credentials: CredentialStore,
    client: Client,
}

impl OpenAiTranscriptionBackend {
    pub fn new(credentials: CredentialStore) -> Result<Self, BackendError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .map_err(|error| BackendError::NetworkFailure(error.to_string()))?;
        Ok(Self {
            credentials,
            client,
        })
    }

    pub fn credentials(&self) -> &CredentialStore {
        &self.credentials
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiTranscriptionResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
}

impl TranscriptionBackend for OpenAiTranscriptionBackend {
    fn name(&self) -> &'static str {
        "openai_cloud"
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionResult, BackendError> {
        let api_key = self.credentials.openai_key()?;
        let started = Instant::now();
        let wav = encode_pcm_wav(&request.audio_samples, request.sample_rate);
        let mut form = multipart::Form::new()
            .text("model", OPENAI_TRANSCRIPTION_MODEL.to_owned())
            .part(
                "file",
                multipart::Part::bytes(wav)
                    .file_name("lyra-audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|error| BackendError::NetworkFailure(error.to_string()))?,
            );
        if let Some(language) = openai_language(request.language) {
            form = form.text("language", language.to_owned());
        }
        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .map_err(|error| BackendError::NetworkFailure(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            warn!(backend = self.name(), status = %status, "cloud transcription request failed");
            return Err(BackendError::NetworkFailure(format!(
                "transcription service returned HTTP {status}"
            )));
        }
        let value = response
            .json::<OpenAiTranscriptionResponse>()
            .map_err(|error| BackendError::NetworkFailure(error.to_string()))?;
        let duration = started.elapsed().as_millis() as u64;
        Ok(TranscriptionResult {
            text: value.text,
            backend: self.name().to_owned(),
            model: OPENAI_TRANSCRIPTION_MODEL.to_owned(),
            language: value.language,
            audio_duration_ms: request.duration_ms(),
            transcription_duration_ms: duration,
            latency_ms: duration,
            fallback_reason: None,
        })
    }
}

#[derive(Clone)]
pub struct TranscriptionRouter {
    local: Arc<dyn TranscriptionBackend>,
    cloud: Arc<dyn TranscriptionBackend>,
    credentials: CredentialStore,
}

impl TranscriptionRouter {
    pub fn new(runtime: RuntimeManager) -> Self {
        let credentials = CredentialStore::default();
        let cloud = OpenAiTranscriptionBackend::new(credentials.clone())
            .expect("cloud client should initialize");
        Self::with_backends(
            Arc::new(LocalNemoBackend::new(runtime)),
            Arc::new(cloud),
            credentials,
        )
    }

    pub fn with_backends(
        local: Arc<dyn TranscriptionBackend>,
        cloud: Arc<dyn TranscriptionBackend>,
        credentials: CredentialStore,
    ) -> Self {
        Self {
            local,
            cloud,
            credentials,
        }
    }

    pub fn credentials(&self) -> &CredentialStore {
        &self.credentials
    }

    pub fn transcribe(
        &self,
        request: &TranscriptionRequest,
        route: TranscriptionRoute,
    ) -> Result<TranscriptionResult, BackendError> {
        match route {
            TranscriptionRoute::LocalOnly => self.local.transcribe(request),
            TranscriptionRoute::CloudOnly => self.cloud.transcribe(request),
            TranscriptionRoute::LocalFirstCloudFallback => match self.local.transcribe(request) {
                Ok(result) => Ok(result),
                Err(local_error) => {
                    let local_reason = local_error.to_string();
                    match self.cloud.transcribe(request) {
                        Ok(mut result) => {
                            result.fallback_reason = Some(local_reason);
                            Ok(result)
                        }
                        Err(cloud_error) if matches!(cloud_error, BackendError::CredentialFailure(_)) => {
                            Err(BackendError::MissingRuntimeModel(format!("local transcription failed: {local_reason}; cloud fallback unavailable because no credential is configured")))
                        }
                        Err(cloud_error) => Err(BackendError::NetworkFailure(format!("local transcription failed: {local_reason}; cloud fallback failed: {cloud_error}"))),
                    }
                }
            },
        }
    }
}

fn openai_language(language: LanguagePreference) -> Option<&'static str> {
    match language {
        LanguagePreference::Auto => None,
        LanguagePreference::En => Some("en"),
        LanguagePreference::El => Some("el"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StubBackend {
        name: &'static str,
        calls: Arc<AtomicUsize>,
        result: Mutex<Result<TranscriptionResult, BackendError>>,
    }

    impl StubBackend {
        fn success(name: &'static str, calls: Arc<AtomicUsize>) -> Self {
            Self {
                name,
                calls,
                result: Mutex::new(Ok(TranscriptionResult {
                    text: "hello".to_owned(),
                    backend: name.to_owned(),
                    model: "test".to_owned(),
                    language: None,
                    audio_duration_ms: 10,
                    transcription_duration_ms: 2,
                    latency_ms: 2,
                    fallback_reason: None,
                })),
            }
        }

        fn failure(name: &'static str, calls: Arc<AtomicUsize>, error: BackendError) -> Self {
            Self {
                name,
                calls,
                result: Mutex::new(Err(error)),
            }
        }
    }

    impl TranscriptionBackend for StubBackend {
        fn name(&self) -> &'static str {
            self.name
        }
        fn transcribe(
            &self,
            _request: &TranscriptionRequest,
        ) -> Result<TranscriptionResult, BackendError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result.lock().unwrap().clone()
        }
    }

    fn request() -> TranscriptionRequest {
        TranscriptionRequest {
            audio_samples: vec![0.0; 160],
            sample_rate: 16_000,
            language: LanguagePreference::Auto,
            request_id: "test".to_owned(),
        }
    }

    #[test]
    fn local_only_never_calls_cloud() {
        let local_calls = Arc::new(AtomicUsize::new(0));
        let cloud_calls = Arc::new(AtomicUsize::new(0));
        let router = TranscriptionRouter::with_backends(
            Arc::new(StubBackend::success("local", local_calls.clone())),
            Arc::new(StubBackend::success("cloud", cloud_calls.clone())),
            CredentialStore::default(),
        );
        assert_eq!(
            router
                .transcribe(&request(), TranscriptionRoute::LocalOnly)
                .unwrap()
                .backend,
            "local"
        );
        assert_eq!(local_calls.load(Ordering::SeqCst), 1);
        assert_eq!(cloud_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn local_failure_marks_successful_cloud_fallback() {
        let local_calls = Arc::new(AtomicUsize::new(0));
        let cloud_calls = Arc::new(AtomicUsize::new(0));
        let router = TranscriptionRouter::with_backends(
            Arc::new(StubBackend::failure(
                "local",
                local_calls.clone(),
                BackendError::GpuFailure("test".to_owned()),
            )),
            Arc::new(StubBackend::success("cloud", cloud_calls.clone())),
            CredentialStore::default(),
        );
        let result = router
            .transcribe(&request(), TranscriptionRoute::LocalFirstCloudFallback)
            .unwrap();
        assert_eq!(result.backend, "cloud");
        assert!(result.fallback_reason.is_some());
        assert_eq!(local_calls.load(Ordering::SeqCst), 1);
        assert_eq!(cloud_calls.load(Ordering::SeqCst), 1);
    }
}

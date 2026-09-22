use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::{multipart, Client};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    audio::encode_pcm_wav,
    domain::{
        BackendError, LanguagePreference, TranscriptionRequest, TranscriptionResult,
        TranscriptionRoute,
    },
    native_nemo::NativeNemoRecognizer,
    runtime::RuntimeManager,
    settings::ModelWarmPolicy,
    streaming::StreamingRequest,
};

pub const OPENAI_TRANSCRIPTION_MODEL: &str = "gpt-4o-mini-transcribe";

pub trait TranscriptionBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionResult, BackendError>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelLoadState {
    Loading,
    Ready,
    Unloaded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeStatus {
    pub state: ModelLoadState,
    pub detail: Option<String>,
}

impl Default for ModelRuntimeStatus {
    fn default() -> Self {
        Self {
            state: ModelLoadState::Unloaded,
            detail: None,
        }
    }
}

pub struct LocalNemoBackend {
    runtime: RuntimeManager,
    recognizer: Mutex<Option<LoadedRecognizer>>,
    policy: Mutex<ModelWarmPolicy>,
    status: Mutex<ModelRuntimeStatus>,
    last_used: Mutex<Instant>,
    status_sink: Mutex<Option<Arc<dyn Fn(ModelRuntimeStatus) + Send + Sync>>>,
}

#[derive(Clone)]
struct LoadedRecognizer {
    variant: String,
    recognizer: NativeNemoRecognizer,
}

impl LocalNemoBackend {
    pub fn new(runtime: RuntimeManager) -> Self {
        Self {
            runtime,
            recognizer: Mutex::new(None),
            policy: Mutex::new(ModelWarmPolicy::AlwaysWarm),
            status: Mutex::new(ModelRuntimeStatus::default()),
            last_used: Mutex::new(Instant::now()),
            status_sink: Mutex::new(None),
        }
    }

    pub fn status(&self) -> ModelRuntimeStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| ModelRuntimeStatus {
                state: ModelLoadState::Error,
                detail: Some("model runtime status lock poisoned".to_owned()),
            })
    }

    pub fn reconcile_policy(self: &Arc<Self>, policy: ModelWarmPolicy) {
        let changed = self
            .policy
            .lock()
            .map(|mut current| {
                if *current == policy {
                    false
                } else {
                    *current = policy;
                    true
                }
            })
            .unwrap_or(true);
        if !changed
            && matches!(
                self.status().state,
                ModelLoadState::Ready | ModelLoadState::Loading
            )
        {
            return;
        }
        if policy == ModelWarmPolicy::NeverKeepWarm {
            self.unload_if_idle();
        } else if self.runtime.is_ready() {
            let backend = Arc::clone(self);
            self.set_status(ModelLoadState::Loading, None);
            let _ = thread::Builder::new()
                .name("lyra-model-preload".to_owned())
                .spawn(move || {
                    if let Err(error) = backend.ensure_loaded() {
                        backend.set_status(ModelLoadState::Error, Some(error.to_string()));
                    }
                });
        } else {
            self.set_status(
                ModelLoadState::Unloaded,
                Some("verified runtime and model artifacts are required".to_owned()),
            );
        }
    }

    pub fn start_inactivity_monitor(self: &Arc<Self>) {
        let backend = Arc::clone(self);
        let _ = thread::Builder::new()
            .name("lyra-model-lifecycle".to_owned())
            .spawn(move || loop {
                thread::sleep(Duration::from_secs(15));
                backend.unload_if_idle();
            });
    }

    fn set_status(&self, state: ModelLoadState, detail: Option<String>) {
        let next = ModelRuntimeStatus { state, detail };
        if let Ok(mut status) = self.status.lock() {
            *status = next.clone();
        }
        if let Ok(sink) = self.status_sink.lock() {
            if let Some(sink) = sink.as_ref() {
                sink(next);
            }
        }
    }

    fn policy(&self) -> ModelWarmPolicy {
        self.policy
            .lock()
            .map(|policy| *policy)
            .unwrap_or(ModelWarmPolicy::NeverKeepWarm)
    }

    fn ensure_loaded(&self) -> Result<(), BackendError> {
        let status = self.runtime.status();
        if !status.runtime_installed
            || !status.runtime_verified
            || !status.model_installed
            || !status.model_verified
        {
            let error = BackendError::MissingRuntimeModel(
                "install and verify the local runtime and Parakeet model in setup".to_owned(),
            );
            self.set_status(ModelLoadState::Unloaded, Some(error.to_string()));
            return Err(error);
        }
        let variant = status.runtime_variant.as_deref().unwrap_or("cuda");
        self.ensure_variant_loaded(variant)
    }

    fn ensure_variant_loaded(&self, variant: &str) -> Result<(), BackendError> {
        if !self.runtime.is_variant_ready(variant) {
            return Err(BackendError::MissingRuntimeModel(format!(
                "verified {variant} runtime is not available"
            )));
        }
        let mut recognizer = self
            .recognizer
            .lock()
            .map_err(|_| BackendError::Unsupported("local recognizer lock poisoned".to_owned()))?;
        if recognizer
            .as_ref()
            .is_some_and(|loaded| loaded.variant != variant)
        {
            recognizer.take();
        }
        if recognizer.is_none() {
            let gpu_index = if variant == "cpu" { -1 } else { 0 };
            let loaded = match NativeNemoRecognizer::load(
                &self.runtime.runtime_path(variant),
                &self.runtime.model_path(),
                gpu_index,
            ) {
                Ok(loaded) => loaded,
                Err(error) => {
                    self.set_status(ModelLoadState::Error, Some(error.to_string()));
                    return Err(error);
                }
            };
            let runtime_version = loaded.version().unwrap_or_else(|| "unknown".to_owned());
            info!(
                backend = self.name(),
                variant,
                model = crate::runtime::MODEL_NAME,
                runtime_version,
                "local recognizer warmed"
            );
            *recognizer = Some(LoadedRecognizer {
                variant: variant.to_owned(),
                recognizer: loaded,
            });
        }
        if let Ok(mut last_used) = self.last_used.lock() {
            *last_used = Instant::now();
        }
        self.set_status(ModelLoadState::Ready, None);
        Ok(())
    }

    fn loaded_variant(&self) -> Option<String> {
        self.recognizer
            .lock()
            .ok()
            .and_then(|recognizer| recognizer.as_ref().map(|loaded| loaded.variant.clone()))
    }

    /// Start one optional native streaming session without exposing the
    /// recognizer handle or native lifetime to callers.  The cloned owner in
    /// `NativeNemoStreamingSession` keeps the loaded recognizer alive until
    /// the stream is closed, even after this lock is released.
    pub fn start_stream(
        &self,
        request: StreamingRequest,
    ) -> Result<crate::native_nemo::NativeNemoStreamingSession, BackendError> {
        self.ensure_loaded()?;
        let recognizer = self
            .recognizer
            .lock()
            .map_err(|_| BackendError::Unsupported("local recognizer lock poisoned".to_owned()))?
            .as_ref()
            .ok_or_else(|| {
                BackendError::MissingRuntimeModel("local recognizer did not initialize".to_owned())
            })?
            .recognizer
            .clone();
        recognizer.start_stream(request)
    }

    pub fn streaming_available(&self) -> bool {
        self.recognizer
            .lock()
            .ok()
            .and_then(|recognizer| {
                recognizer
                    .as_ref()
                    .map(|loaded| loaded.recognizer.streaming_available())
            })
            .unwrap_or(false)
    }

    fn recognize_loaded(&self, request: &TranscriptionRequest) -> Result<String, BackendError> {
        let recognizer = self
            .recognizer
            .lock()
            .map_err(|_| BackendError::Unsupported("local recognizer lock poisoned".to_owned()))?;
        recognizer
            .as_ref()
            .ok_or_else(|| {
                BackendError::MissingRuntimeModel("local recognizer did not initialize".to_owned())
            })?
            .recognizer
            .recognize(request)
    }

    fn unload_if_idle(&self) {
        let policy = self.policy();
        let Some(timeout) = model_inactivity_timeout(policy) else {
            return;
        };
        let idle = self
            .last_used
            .lock()
            .map(|last_used| last_used.elapsed())
            .unwrap_or_default();
        if idle < timeout {
            return;
        }
        if let Ok(mut recognizer) = self.recognizer.lock() {
            if recognizer.take().is_some() {
                info!(?policy, "local recognizer unloaded after inactivity policy");
            }
            self.set_status(ModelLoadState::Unloaded, None);
        }
    }
}

pub fn model_inactivity_timeout(policy: ModelWarmPolicy) -> Option<Duration> {
    match policy {
        ModelWarmPolicy::AlwaysWarm => None,
        ModelWarmPolicy::UnloadAfter5Minutes => Some(Duration::from_secs(5 * 60)),
        ModelWarmPolicy::UnloadAfter15Minutes => Some(Duration::from_secs(15 * 60)),
        ModelWarmPolicy::UnloadAfter30Minutes => Some(Duration::from_secs(30 * 60)),
        ModelWarmPolicy::NeverKeepWarm => Some(Duration::ZERO),
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
        self.ensure_loaded()?;
        let active_variant = self
            .loaded_variant()
            .unwrap_or_else(|| "unknown".to_owned());
        let text = match self.recognize_loaded(request) {
            Ok(text) => text,
            Err(error) if matches!(&error, BackendError::LocalRecognitionFailure(_)) => {
                let fallback_variants = self.runtime.verified_fallbacks(&active_variant);
                if fallback_variants.is_empty() {
                    self.set_status(ModelLoadState::Error, Some(error.to_string()));
                    if self.policy() == ModelWarmPolicy::NeverKeepWarm {
                        self.unload_if_idle();
                    }
                    return Err(error);
                }

                let mut detail = error.to_string();
                let mut recovered_text = None;
                for fallback_variant in fallback_variants {
                    warn!(
                        from = %active_variant,
                        to = %fallback_variant,
                        error = %error,
                        "local runtime recognition failed; retrying with verified fallback"
                    );
                    match self
                        .ensure_variant_loaded(&fallback_variant)
                        .and_then(|_| self.recognize_loaded(request))
                    {
                        Ok(text) => {
                            info!(
                                from = %active_variant,
                                to = %fallback_variant,
                                "local recognition recovered with fallback runtime"
                            );
                            recovered_text = Some(text);
                            break;
                        }
                        Err(fallback_error) => {
                            detail.push_str(&format!(
                                "; {fallback_variant} fallback failed: {fallback_error}"
                            ));
                        }
                    }
                }

                if let Some(text) = recovered_text {
                    text
                } else {
                    let combined = BackendError::LocalRecognitionFailure(detail);
                    self.set_status(ModelLoadState::Error, Some(combined.to_string()));
                    if self.policy() == ModelWarmPolicy::NeverKeepWarm {
                        self.unload_if_idle();
                    }
                    return Err(combined);
                }
            }
            Err(error) => {
                self.set_status(ModelLoadState::Error, Some(error.to_string()));
                if self.policy() == ModelWarmPolicy::NeverKeepWarm {
                    self.unload_if_idle();
                }
                return Err(error);
            }
        };
        let transcription_duration_ms = started.elapsed().as_millis() as u64;
        let result = TranscriptionResult {
            text,
            backend: self.name().to_owned(),
            model: crate::runtime::MODEL_NAME.to_owned(),
            language: None,
            audio_duration_ms: request.duration_ms(),
            transcription_duration_ms,
            latency_ms: transcription_duration_ms,
            fallback_reason: None,
        };
        if self.policy() == ModelWarmPolicy::NeverKeepWarm {
            self.unload_if_idle();
        }
        Ok(result)
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
    model_controller: Option<Arc<LocalNemoBackend>>,
}

impl TranscriptionRouter {
    pub fn new(runtime: RuntimeManager) -> Self {
        let credentials = CredentialStore::default();
        let cloud = OpenAiTranscriptionBackend::new(credentials.clone())
            .expect("cloud client should initialize");
        let local = Arc::new(LocalNemoBackend::new(runtime));
        let router = Self {
            local: local.clone(),
            cloud: Arc::new(cloud),
            credentials,
            model_controller: Some(local.clone()),
        };
        local.start_inactivity_monitor();
        router
    }

    #[cfg(test)]
    pub fn with_backends(
        local: Arc<dyn TranscriptionBackend>,
        cloud: Arc<dyn TranscriptionBackend>,
        credentials: CredentialStore,
    ) -> Self {
        Self {
            local,
            cloud,
            credentials,
            model_controller: None,
        }
    }

    pub fn model_status(&self) -> ModelRuntimeStatus {
        self.model_controller
            .as_ref()
            .map(|controller| controller.status())
            .unwrap_or_default()
    }

    pub fn set_model_status_sink(&self, sink: Arc<dyn Fn(ModelRuntimeStatus) + Send + Sync>) {
        if let Some(controller) = &self.model_controller {
            if let Ok(mut current) = controller.status_sink.lock() {
                *current = Some(sink);
            }
        }
    }

    pub fn reconcile_model_policy(&self, policy: ModelWarmPolicy) {
        if let Some(controller) = &self.model_controller {
            controller.reconcile_policy(policy);
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
                    if matches!(&local_error, BackendError::EmptyRecording) {
                        return Err(local_error);
                    }
                    let local_reason = local_error.to_string();
                    match self.cloud.transcribe(request) {
                        Ok(mut result) => {
                            result.fallback_reason = Some(local_reason);
                            Ok(result)
                        }
                        Err(cloud_error) if matches!(cloud_error, BackendError::CredentialFailure(_)) => {
                            Err(BackendError::CloudFallbackUnavailable(local_reason))
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
    fn warm_policies_have_explicit_inactivity_contracts() {
        assert_eq!(model_inactivity_timeout(ModelWarmPolicy::AlwaysWarm), None);
        assert_eq!(
            model_inactivity_timeout(ModelWarmPolicy::UnloadAfter5Minutes),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            model_inactivity_timeout(ModelWarmPolicy::UnloadAfter15Minutes),
            Some(Duration::from_secs(900))
        );
        assert_eq!(
            model_inactivity_timeout(ModelWarmPolicy::UnloadAfter30Minutes),
            Some(Duration::from_secs(1_800))
        );
        assert_eq!(
            model_inactivity_timeout(ModelWarmPolicy::NeverKeepWarm),
            Some(Duration::ZERO)
        );
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
    fn empty_local_recording_does_not_attempt_cloud_fallback() {
        let local_calls = Arc::new(AtomicUsize::new(0));
        let cloud_calls = Arc::new(AtomicUsize::new(0));
        let router = TranscriptionRouter::with_backends(
            Arc::new(StubBackend::failure(
                "local",
                local_calls.clone(),
                BackendError::EmptyRecording,
            )),
            Arc::new(StubBackend::success("cloud", cloud_calls.clone())),
            CredentialStore::default(),
        );
        assert!(matches!(
            router.transcribe(&request(), TranscriptionRoute::LocalFirstCloudFallback),
            Err(BackendError::EmptyRecording)
        ));
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
                BackendError::LocalRecognitionFailure("test".to_owned()),
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

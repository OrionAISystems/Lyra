use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    PushToTalk,
    Toggle,
}

impl Default for ActivationMode {
    fn default() -> Self {
        Self::PushToTalk
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DictationState {
    Idle,
    Listening,
    Processing,
    Success,
    Error,
}

impl Default for DictationState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePreference {
    Auto,
    En,
    El,
}

impl Default for LanguagePreference {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionRoute {
    LocalOnly,
    LocalFirstCloudFallback,
    CloudOnly,
}

impl Default for TranscriptionRoute {
    fn default() -> Self {
        Self::LocalFirstCloudFallback
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivationSource {
    Hotkey,
    Bar,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutEdge {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationAction {
    Start,
    Stop,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct ActivationController {
    mode: ActivationMode,
    active: bool,
}

impl ActivationController {
    pub fn new(mode: ActivationMode) -> Self {
        Self {
            mode,
            active: false,
        }
    }

    pub fn set_mode(&mut self, mode: ActivationMode) {
        self.mode = mode;
        self.active = false;
    }

    pub fn handle(&mut self, edge: ShortcutEdge) -> ActivationAction {
        match (self.mode, edge) {
            (ActivationMode::PushToTalk, ShortcutEdge::Pressed) if !self.active => {
                self.active = true;
                ActivationAction::Start
            }
            (ActivationMode::PushToTalk, ShortcutEdge::Released) if self.active => {
                self.active = false;
                ActivationAction::Stop
            }
            (ActivationMode::Toggle, ShortcutEdge::Pressed) if !self.active => {
                self.active = true;
                ActivationAction::Start
            }
            (ActivationMode::Toggle, ShortcutEdge::Pressed) if self.active => {
                self.active = false;
                ActivationAction::Stop
            }
            _ => ActivationAction::Ignore,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "message")]
pub enum BackendError {
    MissingRuntimeModel(String),
    DeviceFailure(String),
    PermissionDenied(String),
    EmptyRecording,
    MaximumDuration,
    StreamFailure(String),
    Timeout,
    GpuFailure(String),
    CredentialFailure(String),
    NetworkFailure(String),
    CloudFallbackUnavailable(String),
    Cancellation,
    Unsupported(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRuntimeModel(message) => {
                write!(f, "local runtime or model unavailable: {message}")
            }
            Self::DeviceFailure(message) => write!(f, "audio device failure: {message}"),
            Self::PermissionDenied(message) => write!(f, "permission denied: {message}"),
            Self::EmptyRecording => write!(f, "no audio was captured"),
            Self::MaximumDuration => write!(f, "maximum dictation duration reached"),
            Self::StreamFailure(message) => write!(f, "audio stream failure: {message}"),
            Self::Timeout => write!(f, "transcription timed out"),
            Self::GpuFailure(message) => write!(f, "GPU recognition failed: {message}"),
            Self::CredentialFailure(message) => write!(f, "cloud credential failure: {message}"),
            Self::NetworkFailure(message) => write!(f, "cloud network failure: {message}"),
            Self::CloudFallbackUnavailable(message) => write!(
                f,
                "local transcription failed: {message}; cloud fallback unavailable because no credential is configured"
            ),
            Self::Cancellation => write!(f, "dictation cancelled"),
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
        }
    }
}

impl std::error::Error for BackendError {}

#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    pub audio_samples: Vec<f32>,
    pub sample_rate: u32,
    pub language: LanguagePreference,
    pub request_id: String,
}

impl TranscriptionRequest {
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        ((self.audio_samples.len() as u64) * 1_000) / u64::from(self.sample_rate)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub backend: String,
    pub model: String,
    pub language: Option<String>,
    pub audio_duration_ms: u64,
    pub transcription_duration_ms: u64,
    pub latency_ms: u64,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub state: DictationState,
    pub duration_ms: u64,
    pub input_level: f32,
    pub request_id: Option<String>,
    pub error: Option<String>,
    pub transcript: Option<String>,
    pub backend: Option<String>,
    pub fallback_reason: Option<String>,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            state: DictationState::Idle,
            duration_ms: 0,
            input_level: 0.0,
            request_id: None,
            error: None,
            transcript: None,
            backend: None,
            fallback_reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    Start,
    Stop,
    Cancel,
    Success,
    Failure,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitionError {
    pub state: DictationState,
    pub event: SessionEvent,
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot apply {:?} while in {:?}", self.event, self.state)
    }
}

impl std::error::Error for StateTransitionError {}

#[derive(Debug, Clone)]
pub struct DictationStateMachine {
    state: DictationState,
}

impl Default for DictationStateMachine {
    fn default() -> Self {
        Self {
            state: DictationState::Idle,
        }
    }
}

impl DictationStateMachine {
    pub fn state(&self) -> DictationState {
        self.state
    }

    pub fn transition(
        &mut self,
        event: SessionEvent,
    ) -> Result<DictationState, StateTransitionError> {
        let next = match (self.state, event) {
            (DictationState::Idle, SessionEvent::Start) => DictationState::Listening,
            (DictationState::Listening, SessionEvent::Stop) => DictationState::Processing,
            (DictationState::Listening, SessionEvent::Cancel) => DictationState::Idle,
            (DictationState::Listening, SessionEvent::Failure) => DictationState::Error,
            (DictationState::Processing, SessionEvent::Success) => DictationState::Success,
            (DictationState::Processing, SessionEvent::Failure) => DictationState::Error,
            (DictationState::Processing, SessionEvent::Cancel) => DictationState::Idle,
            (DictationState::Success, SessionEvent::Reset) => DictationState::Idle,
            (DictationState::Error, SessionEvent::Reset) => DictationState::Idle,
            (DictationState::Error, SessionEvent::Start) => DictationState::Listening,
            _ => {
                return Err(StateTransitionError {
                    state: self.state,
                    event,
                })
            }
        };
        self.state = next;
        Ok(next)
    }
}

pub fn post_process(raw: &str, preceding_character: Option<char>) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let text = normalized.trim().to_string();
    if text.is_empty() {
        return text;
    }

    let starts_with_punctuation = text.chars().next().is_some_and(|character| {
        character.is_ascii_punctuation() && !matches!(character, '\'' | '-')
    });
    let word_like = text.chars().any(|character| character.is_alphanumeric())
        && text.chars().all(|character| !character.is_control());
    let needs_separator = word_like
        && !starts_with_punctuation
        && preceding_character.is_some_and(|character| character.is_alphanumeric());

    if needs_separator {
        format!(" {text}")
    } else {
        text
    }
}

pub trait PostProcessor: Send + Sync {
    fn process(&self, raw: &str, preceding_character: Option<char>) -> String;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ConservativePostProcessor;

impl PostProcessor for ConservativePostProcessor {
    fn process(&self, raw: &str, preceding_character: Option<char>) -> String {
        post_process(raw, preceding_character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_to_talk_requires_key_up_to_stop() {
        let mut controller = ActivationController::new(ActivationMode::PushToTalk);
        assert_eq!(
            controller.handle(ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(
            controller.handle(ShortcutEdge::Pressed),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle(ShortcutEdge::Released),
            ActivationAction::Stop
        );
        assert_eq!(
            controller.handle(ShortcutEdge::Released),
            ActivationAction::Ignore
        );
    }

    #[test]
    fn toggle_changes_on_each_press_and_ignores_release() {
        let mut controller = ActivationController::new(ActivationMode::Toggle);
        assert_eq!(
            controller.handle(ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(
            controller.handle(ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle(ShortcutEdge::Pressed),
            ActivationAction::Stop
        );
    }

    #[test]
    fn state_machine_rejects_invalid_transition() {
        let mut machine = DictationStateMachine::default();
        assert!(machine.transition(SessionEvent::Stop).is_err());
        assert_eq!(
            machine.transition(SessionEvent::Start).unwrap(),
            DictationState::Listening
        );
        assert_eq!(
            machine.transition(SessionEvent::Cancel).unwrap(),
            DictationState::Idle
        );
    }

    #[test]
    fn post_process_preserves_punctuation_and_only_adds_word_spacing() {
        assert_eq!(post_process("  hello\r\n", Some('a')), " hello");
        assert_eq!(post_process("  world!  ", Some('!')), "world!");
        assert_eq!(post_process("...", Some('a')), "...");
        assert_eq!(post_process("\r\n", Some('a')), "");
    }

    #[test]
    fn frontend_payloads_use_camel_case_field_names() {
        let snapshot = SessionSnapshot {
            duration_ms: 1_250,
            input_level: 0.4,
            request_id: Some("request-1".to_owned()),
            fallback_reason: Some("local unavailable".to_owned()),
            ..SessionSnapshot::default()
        };
        let snapshot_json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(snapshot_json["durationMs"], 1_250);
        assert!((snapshot_json["inputLevel"].as_f64().unwrap() - 0.4).abs() < 1e-6);
        assert_eq!(snapshot_json["requestId"], "request-1");
        assert!(snapshot_json.get("duration_ms").is_none());

        let result = TranscriptionResult {
            text: "hello".to_owned(),
            backend: "local".to_owned(),
            model: "parakeet".to_owned(),
            language: Some("en".to_owned()),
            audio_duration_ms: 1_250,
            transcription_duration_ms: 300,
            latency_ms: 350,
            fallback_reason: None,
        };
        let result_json = serde_json::to_value(result).unwrap();
        assert_eq!(result_json["audioDurationMs"], 1_250);
        assert_eq!(result_json["transcriptionDurationMs"], 300);
        assert_eq!(result_json["latencyMs"], 350);
        assert!(result_json.get("audio_duration_ms").is_none());
    }
}

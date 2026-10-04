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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionEntry {
    pub source: String,
    pub replacement: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub case_sensitive: bool,
}

fn default_true() -> bool {
    true
}

impl CorrectionEntry {
    pub fn new(source: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            replacement: replacement.into(),
            enabled: true,
            case_sensitive: false,
        }
    }
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
    active_mode: Option<ActivationMode>,
}

impl ActivationController {
    pub fn new(mode: ActivationMode) -> Self {
        Self {
            mode,
            active_mode: None,
        }
    }

    pub fn set_mode(&mut self, mode: ActivationMode) {
        self.mode = mode;
    }

    pub fn reset(&mut self) {
        self.active_mode = None;
    }

    pub fn handle(&mut self, edge: ShortcutEdge) -> ActivationAction {
        match (self.active_mode, edge) {
            (None, ShortcutEdge::Pressed) => {
                self.active_mode = Some(self.mode);
                ActivationAction::Start
            }
            (Some(ActivationMode::PushToTalk), ShortcutEdge::Released) => {
                self.active_mode = None;
                ActivationAction::Stop
            }
            (Some(ActivationMode::Toggle), ShortcutEdge::Pressed) => {
                self.active_mode = None;
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
    LocalRecognitionFailure(String),
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
            Self::LocalRecognitionFailure(message) => {
                write!(f, "local recognition failed: {message}")
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEndpoint {
    Manual,
    MaximumDuration,
    Silence,
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
    pub backend: Option<String>,
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub destination: Option<SessionDestination>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionDestination {
    Dictation,
    Scratchpad,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            state: DictationState::Idle,
            duration_ms: 0,
            input_level: 0.0,
            request_id: None,
            error: None,
            backend: None,
            fallback_reason: None,
            destination: None,
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

#[derive(Debug, Clone, Copy)]
pub struct TextProcessingOptions<'a> {
    pub formatting_commands_enabled: bool,
    pub self_correction_enabled: bool,
    pub corrections: &'a [CorrectionEntry],
    pub snippets: &'a [crate::personalization::SnippetEntry],
}

pub fn process_transcription_text(
    raw: &str,
    preceding_character: Option<char>,
    options: TextProcessingOptions<'_>,
) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut text = normalized.trim().to_owned();
    if text.is_empty() {
        return text;
    }

    if options.self_correction_enabled {
        text = apply_explicit_self_correction(&text);
    }

    if options.formatting_commands_enabled {
        text = replace_line_break_commands(&text, &[("new paragraph", "\n\n"), ("new line", "\n")]);
        let mut formatting = vec![("open quote", "\""), ("close quote", "\"")];
        formatting.sort_by_key(|(source, _)| std::cmp::Reverse(source.len()));
        text = replace_phrases(&text, &formatting, true);
        text = replace_spoken_punctuation(&text, "comma", ',');
        text = replace_spoken_punctuation(&text, "period", '.');
        text = normalize_formatting_spacing(&text);
        text = crate::quantities::normalize(&text);
    }

    let mut corrections = options
        .corrections
        .iter()
        .filter(|entry| entry.enabled && !entry.source.trim().is_empty())
        .collect::<Vec<_>>();
    corrections.sort_by_key(|entry| std::cmp::Reverse(entry.source.chars().count()));
    for entry in corrections {
        text = replace_phrase(
            &text,
            entry.source.trim(),
            &entry.replacement,
            entry.case_sensitive,
        );
    }

    text = crate::personalization::apply_snippets(&text, options.snippets);

    post_process(&text, preceding_character)
}

fn apply_explicit_self_correction(text: &str) -> String {
    const CUES: [&str; 4] = ["scratch that", "actually", "i mean", "make that"];
    let lower = text.to_ascii_lowercase();
    let Some((cue_start, cue_end)) = CUES
        .iter()
        .filter_map(|cue| {
            lower
                .rfind(cue)
                .filter(|start| phrase_boundary(&lower, *start, *start + cue.len()))
                .map(|start| (start, start + cue.len()))
        })
        .max_by_key(|(start, _)| *start)
    else {
        return text.to_owned();
    };
    let replacement = text[cue_end..]
        .trim_start_matches(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ':' | '-' | '—')
        })
        .trim();
    if replacement.is_empty() {
        return text.to_owned();
    }
    let prefix = &text[..cue_start];
    let clause_start = prefix
        .rfind(['.', '!', '?', '\n'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut output = String::with_capacity(text.len());
    output.push_str(prefix[..clause_start].trim_end());
    if !output.is_empty() && !output.ends_with([' ', '\n']) {
        output.push(' ');
    }
    // Preserve sentence casing when the recognizer lowercases the replacement
    // after a correction cue. Mixed-case names remain the speaker's text.
    let original_clause = &prefix[clause_start..];
    if original_clause[sentence_content_start(original_clause)..]
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
    {
        output.push_str(&capitalize_sentence_start(replacement));
    } else {
        output.push_str(replacement);
    }
    output.trim().to_owned()
}

fn replace_phrases(text: &str, replacements: &[(&str, &str)], case_insensitive: bool) -> String {
    let mut result = text.to_owned();
    for (source, replacement) in replacements {
        result = replace_phrase(&result, source, replacement, !case_insensitive);
    }
    result
}

pub(crate) fn replace_phrase_public(
    text: &str,
    source: &str,
    replacement: &str,
    case_sensitive: bool,
) -> String {
    replace_phrase(text, source, replacement, case_sensitive)
}

fn replace_phrase(text: &str, source: &str, replacement: &str, case_sensitive: bool) -> String {
    let source = source.trim();
    if source.is_empty() || text.is_empty() || source.len() > text.len() {
        return text.to_owned();
    }
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    while cursor < text.len() {
        let Some((start, end)) = find_phrase(text, cursor, source, case_sensitive) else {
            result.push_str(&text[cursor..]);
            break;
        };
        result.push_str(&text[cursor..start]);
        result.push_str(replacement);
        cursor = end;
    }
    result
}

fn find_phrase(
    text: &str,
    cursor: usize,
    source: &str,
    case_sensitive: bool,
) -> Option<(usize, usize)> {
    for (offset, _) in text[cursor..].char_indices() {
        let start = cursor + offset;
        let end = start + source.len();
        let Some(candidate) = text.get(start..end) else {
            continue;
        };
        if phrase_boundary(text, start, end)
            && if case_sensitive {
                candidate == source
            } else {
                candidate.eq_ignore_ascii_case(source)
            }
        {
            return Some((start, end));
        }
    }
    None
}

fn phrase_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|character| !character.is_alphanumeric())
        && after.is_none_or(|character| !character.is_alphanumeric())
}

fn replace_line_break_commands(text: &str, commands: &[(&str, &str)]) -> String {
    let mut result = text.to_owned();
    for (source, replacement) in commands {
        result = replace_phrase_and_following_punctuation(&result, source, replacement);
    }
    result
}

// ASR may attach sentence punctuation to a spoken line-break command.
fn replace_phrase_and_following_punctuation(text: &str, source: &str, replacement: &str) -> String {
    let source = source.trim();
    if source.is_empty() || text.is_empty() || source.len() > text.len() {
        return text.to_owned();
    }

    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some((start, end)) = find_phrase(text, cursor, source, false) {
        result.push_str(&text[cursor..start]);
        result.push_str(replacement);
        cursor = consume_following_punctuation(text, end);
    }
    result.push_str(&text[cursor..]);
    result
}

fn consume_following_punctuation(text: &str, end: usize) -> usize {
    let mut punctuation_start = end;
    while let Some(character) = text[punctuation_start..].chars().next() {
        if !matches!(character, ' ' | '\t') {
            break;
        }
        punctuation_start += character.len_utf8();
    }

    let mut cursor = punctuation_start;
    while let Some(character) = text[cursor..].chars().next() {
        if !matches!(character, '.' | ',' | ';' | ':' | '!' | '?') {
            break;
        }
        cursor += character.len_utf8();
    }
    if cursor == punctuation_start {
        return end;
    }

    while let Some(character) = text[cursor..].chars().next() {
        if !matches!(character, ' ' | '\t') {
            break;
        }
        cursor += character.len_utf8();
    }
    cursor
}

fn sentence_content_start(text: &str) -> usize {
    text.char_indices()
        .find(|(_, c)| {
            !c.is_whitespace()
                && !matches!(
                    c,
                    '\'' | '"' | '“' | '”' | '‘' | '’' | '(' | ')' | '[' | ']' | '{' | '}'
                )
        })
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn capitalize_sentence_start(text: &str) -> String {
    let start = sentence_content_start(text);
    let word = text[start..]
        .split(|c: char| !c.is_alphabetic() && !matches!(c, '\'' | '’'))
        .next()
        .unwrap_or("");
    let Some(first) = word.chars().next().filter(|c| c.is_lowercase()) else {
        return text.to_owned();
    };
    if word.chars().any(char::is_uppercase) {
        return text.to_owned();
    }
    format!(
        "{}{}{}",
        &text[..start],
        first.to_uppercase(),
        &text[start + first.len_utf8()..]
    )
}

// A spoken punctuation cue owns adjacent ASR punctuation, rather than globally
// collapsing dots/commas (which would damage decimals, versions and ellipses).
fn replace_spoken_punctuation(text: &str, source: &str, punctuation: char) -> String {
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut sentence_start = false;
    while let Some((start, end)) = find_phrase(text, cursor, source, false) {
        let prefix = &text[cursor..start];
        if sentence_start {
            result.push_str(&capitalize_sentence_start(prefix));
        } else {
            result.push_str(prefix);
        }
        while result.ends_with([' ', '\t']) {
            result.pop();
        }
        while result.ends_with(['.', ',', ';', ':', '!', '?']) {
            result.pop();
        }
        result.push(punctuation);

        // Like line-break cues, consume ASR punctuation attached to the cue.
        // Retain trailing whitespace: punctuation alone must not join words.
        let mut following = end;
        while text[following..].starts_with([' ', '\t']) {
            following += 1;
        }
        let mut punctuation_end = following;
        while let Some(c) = text[punctuation_end..].chars().next() {
            if !matches!(c, '.' | ',' | ';' | ':' | '!' | '?') {
                break;
            }
            punctuation_end += c.len_utf8();
        }
        cursor = if punctuation_end > following {
            punctuation_end
        } else {
            end
        };
        if text[cursor..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
        {
            result.push(' ');
        }
        sentence_start = punctuation == '.';
    }
    let suffix = &text[cursor..];
    if sentence_start {
        result.push_str(&capitalize_sentence_start(suffix));
    } else {
        result.push_str(suffix);
    }
    result
}

fn normalize_formatting_spacing(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut quote_open = false;
    for character in text.chars() {
        if character == '\n' {
            while output.ends_with(' ') || output.ends_with('\t') {
                output.pop();
            }
            output.push('\n');
        } else if matches!(character, ',' | '.') {
            while output.ends_with(' ') || output.ends_with('\t') {
                output.pop();
            }
            output.push(character);
        } else if character == '"' {
            if quote_open {
                while output.ends_with(' ') || output.ends_with('\t') {
                    output.pop();
                }
            } else if output.ends_with(' ') || output.ends_with('\t') {
                // Keep a natural separator before an opening quote.
            }
            output.push(character);
            quote_open = !quote_open;
        } else if (character == ' ' || character == '\t') && output.ends_with('\n') {
            // Formatting commands own whitespace around a line break.
        } else if quote_open && (character == ' ' || character == '\t') && output.ends_with('"') {
            // Do not leave a space immediately inside an opening quote.
        } else {
            output.push(character);
        }
    }
    output
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
    fn activation_mode_changes_do_not_change_the_active_gesture() {
        let mut push_to_talk = ActivationController::new(ActivationMode::PushToTalk);
        assert_eq!(
            push_to_talk.handle(ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        push_to_talk.set_mode(ActivationMode::Toggle);
        assert_eq!(
            push_to_talk.handle(ShortcutEdge::Released),
            ActivationAction::Stop
        );

        let mut toggle = ActivationController::new(ActivationMode::Toggle);
        assert_eq!(
            toggle.handle(ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        toggle.set_mode(ActivationMode::PushToTalk);
        assert_eq!(
            toggle.handle(ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(toggle.handle(ShortcutEdge::Pressed), ActivationAction::Stop);
        toggle.reset();
        assert_eq!(
            toggle.handle(ShortcutEdge::Pressed),
            ActivationAction::Start
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
    fn formatting_commands_are_whole_phrases_and_deterministic() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        assert_eq!(
            process_transcription_text("hello comma world period", None, options),
            "hello, world."
        );
        assert_eq!(
            process_transcription_text("new paragraph next new line item", None, options),
            "next\nitem"
        );
        assert_eq!(
            process_transcription_text(
                "This is a test first sentence. New paragraph. This is a test second sentence.",
                None,
                options,
            ),
            "This is a test first sentence.\n\nThis is a test second sentence."
        );
        assert_eq!(
            process_transcription_text(
                "This is another test first sentence. New line. This is a test second sentence.",
                None,
                options,
            ),
            "This is another test first sentence.\nThis is a test second sentence."
        );
        assert_eq!(
            process_transcription_text("open quote hello close quote", None, options),
            "\"hello\""
        );
        assert_eq!(
            process_transcription_text("commawheel", None, options),
            "commawheel"
        );
    }

    #[test]
    fn corrections_are_longest_first_and_respect_case_setting() {
        let corrections = vec![
            CorrectionEntry::new("geo", "Geo"),
            CorrectionEntry::new("geo core", "GeoCore"),
            CorrectionEntry {
                source: "Tauri".to_owned(),
                replacement: "Tauri SDK".to_owned(),
                enabled: true,
                case_sensitive: true,
            },
        ];
        let options = TextProcessingOptions {
            formatting_commands_enabled: false,
            self_correction_enabled: true,
            corrections: &corrections,
            snippets: &[],
        };
        assert_eq!(
            process_transcription_text("geo core tauri Tauri", None, options),
            "GeoCore tauri Tauri SDK"
        );
    }

    #[test]
    fn spoken_punctuation_owns_asr_marks_and_sentence_casing() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (raw, expected) in [
            (
                "Hello comma Orion period new line please send the report period.",
                "Hello, Orion.\nPlease send the report.",
            ),
            (
                "Hello comma Orion period new line. Please send the report period.",
                "Hello, Orion.\nPlease send the report.",
            ),
            (
                "Hello, comma, Orion period. next sentence period .",
                "Hello, Orion. Next sentence.",
            ),
            ("Hello comma,world period.next", "Hello, world. Next"),
            ("Hello, period.", "Hello."),
            ("Hello. comma,", "Hello,"),
            ("Hello?! period. next", "Hello. Next"),
            (
                "hello period open quote welcome close quote period.",
                "hello. \"Welcome\".",
            ),
            ("hello period new paragraph élève", "hello.\n\nÉlève"),
            ("hello period iPhone and eBay", "hello. iPhone and eBay"),
            (
                "Send the report on Thursday scratch that send the report on Friday period.",
                "Send the report on Friday.",
            ),
        ] {
            assert_eq!(
                process_transcription_text(raw, None, options),
                expected,
                "{raw}"
            );
        }
    }

    #[test]
    fn punctuation_cleanup_preserves_literal_text_and_user_overrides() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for raw in [
            "Wait... then continue.",
            "Value 12.5, version v0.5.5, example.com.",
            "https://example.com/path?x=12.5",
            "commawheel periodical",
            "Already punctuated. next stays as recognized.",
        ] {
            assert_eq!(process_transcription_text(raw, None, options), raw);
        }
        assert_eq!(
            process_transcription_text("new line item", None, options),
            "item"
        );
        assert_eq!(
            process_transcription_text("notes new line item", None, options),
            "notes\nitem"
        );
        let disabled = TextProcessingOptions {
            formatting_commands_enabled: false,
            self_correction_enabled: false,
            ..options
        };
        assert_eq!(
            process_transcription_text("hello period. next", None, disabled),
            "hello period. next"
        );
        let corrections = [CorrectionEntry::new("next", "eBay")];
        let personalized = TextProcessingOptions {
            corrections: &corrections,
            ..options
        };
        assert_eq!(
            process_transcription_text("hello period next", None, personalized),
            "hello. eBay"
        );
        assert_eq!(
            process_transcription_text("Use Amazon scratch that eBay", None, options),
            "eBay"
        );
    }

    #[test]
    fn quantity_formatting_obeys_the_existing_toggle_and_personalization_order() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        let raw = "The order contains twelve boxes and costs forty five dollars.";
        assert_eq!(
            process_transcription_text(raw, None, options),
            "The order contains 12 boxes and costs $45."
        );
        let disabled = TextProcessingOptions {
            formatting_commands_enabled: false,
            ..options
        };
        assert_eq!(process_transcription_text(raw, None, disabled), raw);
        let corrections = [CorrectionEntry::new("$45", "USD 45")];
        let personalized = TextProcessingOptions {
            corrections: &corrections,
            ..options
        };
        assert_eq!(
            process_transcription_text(raw, None, personalized),
            "The order contains 12 boxes and costs USD 45."
        );
        let text =
            process_transcription_text("hello period twelve point five kilograms", None, options);
        assert_eq!(text, "hello. 12.5 kilograms");
        assert_eq!(process_transcription_text(&text, None, options), text);
    }

    #[test]
    fn explicit_self_correction_replaces_only_the_current_clause() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: false,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        assert_eq!(
            process_transcription_text(
                "We agreed yesterday. We should ship Thursday, actually Friday",
                None,
                options,
            ),
            "We agreed yesterday. Friday"
        );
    }

    #[test]
    fn self_correction_casing_skips_opening_delimiters() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: false,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (source, expected) in [
            ("\"Send it scratch that send this", "Send this"),
            ("(Send it scratch that send this", "Send this"),
            ("[Send it scratch that send this", "Send this"),
            ("“Écris ceci scratch that écris cela", "Écris cela"),
            ("(send it scratch that send this", "send this"),
            ("(Send it scratch that eBay", "eBay"),
            ("Done. (Send it scratch that send this", "Done. Send this"),
            ("(Send it scratch that \"send this\"", "\"Send this\""),
        ] {
            assert_eq!(
                process_transcription_text(source, None, options),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn sentence_casing_preserves_names_with_curly_apostrophes() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (source, expected) in [
            (
                "hello period d’Artagnan arrived",
                "hello. d’Artagnan arrived",
            ),
            (
                "hello period d'Artagnan arrived",
                "hello. d'Artagnan arrived",
            ),
            (
                "Send this scratch that d’Artagnan arrived",
                "d’Artagnan arrived",
            ),
            ("hello period l’esprit", "hello. L’esprit"),
            ("‘five boxes’", "‘5 boxes’"),
        ] {
            assert_eq!(
                process_transcription_text(source, None, options),
                expected,
                "{source}"
            );
        }
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

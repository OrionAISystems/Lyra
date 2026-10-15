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
    active_binding: Option<String>,
    held_bindings: std::collections::HashSet<String>,
}

impl ActivationController {
    pub fn new(mode: ActivationMode) -> Self {
        Self {
            mode,
            active_mode: None,
            active_binding: None,
            held_bindings: std::collections::HashSet::new(),
        }
    }

    pub fn set_mode(&mut self, mode: ActivationMode) {
        self.mode = mode;
    }

    pub fn reset(&mut self) {
        self.active_mode = None;
        self.active_binding = None;
        self.held_bindings.clear();
    }

    pub fn active_mode(&self) -> Option<ActivationMode> {
        self.active_mode
    }

    /// The initiating binding owns the gesture; other bindings cannot stop it.
    pub fn handle_binding(&mut self, binding: &str, edge: ShortcutEdge) -> ActivationAction {
        self.handle_binding_with_mode(binding, self.mode, edge)
    }

    pub fn handle_binding_with_mode(
        &mut self,
        binding: &str,
        mode: ActivationMode,
        edge: ShortcutEdge,
    ) -> ActivationAction {
        match edge {
            ShortcutEdge::Pressed => {
                if !self.held_bindings.insert(binding.to_owned()) {
                    return ActivationAction::Ignore;
                }
            }
            ShortcutEdge::Released => {
                if !self.held_bindings.remove(binding) {
                    return ActivationAction::Ignore;
                }
            }
        }
        match (self.active_mode, edge) {
            (None, ShortcutEdge::Pressed) => {
                self.active_binding = Some(binding.to_owned());
                self.active_mode = Some(mode);
                ActivationAction::Start
            }
            (Some(active_mode), _) if self.active_binding.as_deref() == Some(binding) => {
                if (active_mode == ActivationMode::PushToTalk && edge == ShortcutEdge::Released)
                    || (active_mode == ActivationMode::Toggle && edge == ShortcutEdge::Pressed)
                {
                    self.active_mode = None;
                    self.active_binding = None;
                    ActivationAction::Stop
                } else {
                    ActivationAction::Ignore
                }
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
    let text = trim_transcript_boundary(&normalized).to_string();
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

/// Only insertion policy survives the accessibility query; surrounding text is not retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertionContext {
    pub preceding_character: Option<char>,
    pub continues_sentence: bool,
}

impl InsertionContext {
    pub fn from_prefix(prefix: &str) -> Self {
        let preceding_character = prefix.chars().next_back();
        let significant = prefix.chars().rev().find(|c| {
            !matches!(
                c,
                ' ' | '\t'
                    | '\u{a0}'
                    | '"'
                    | '\''
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
            )
        });
        Self {
            preceding_character,
            continues_sentence: significant.is_some_and(|c| {
                !matches!(c, '.' | '!' | '?' | '\n' | '\r' | '。' | '！' | '？' | '…')
            }),
        }
    }
}

pub fn process_transcription_text(
    raw: &str,
    preceding_character: Option<char>,
    options: TextProcessingOptions<'_>,
) -> String {
    process_transcription_text_with_context(
        raw,
        preceding_character.map(|character| InsertionContext {
            preceding_character: Some(character),
            continues_sentence: false,
        }),
        &[],
        options,
    )
}

pub fn process_transcription_text_with_context(
    raw: &str,
    context: Option<InsertionContext>,
    words: &[crate::personalization::WordEntry],
    options: TextProcessingOptions<'_>,
) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut text = trim_transcript_boundary(&normalized).to_owned();
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
        if context.is_some_and(|context| context.continues_sentence) {
            text = lowercase_continuation_start(&text, words, options);
        } else if !start_has_personalization(&text, words, options) {
            text = capitalize_dictation_start(&text);
        }
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

    post_process(
        &text,
        context.and_then(|context| context.preceding_character),
    )
}

fn start_has_personalization(
    text: &str,
    words: &[crate::personalization::WordEntry],
    options: TextProcessingOptions<'_>,
) -> bool {
    let start = sentence_content_start(text);
    let starts_with_phrase = |phrase: &str, case_sensitive| {
        find_phrase(text, start, phrase.trim(), case_sensitive)
            .is_some_and(|(found, _)| found == start)
    };
    words.iter().any(|word| {
        word.enabled
            && !word.phrase.trim().is_empty()
            && word.phrase.chars().any(char::is_uppercase)
            && starts_with_phrase(&word.phrase, false)
    }) || options.corrections.iter().any(|entry| {
        entry.enabled
            && !entry.source.trim().is_empty()
            && starts_with_phrase(&entry.source, entry.case_sensitive)
    }) || options.snippets.iter().any(|entry| {
        entry.enabled
            && !entry.trigger.trim().is_empty()
            && starts_with_phrase(&entry.trigger, entry.case_sensitive)
    })
}

fn lowercase_continuation_start(
    text: &str,
    words: &[crate::personalization::WordEntry],
    options: TextProcessingOptions<'_>,
) -> String {
    // Explicit new-line commands start a fresh block. Later personalization owns its casing.
    if text.starts_with(['\n', '\r']) || start_has_personalization(text, words, options) {
        return text.to_owned();
    }
    let start = sentence_content_start(text);
    let word = text[start..]
        .split(|c: char| !c.is_alphabetic() && !matches!(c, '\'' | '’' | '-'))
        .next()
        .unwrap_or("");
    let Some(first) = word.chars().next().filter(|c| c.is_uppercase()) else {
        return text.to_owned();
    };
    let rest = &word[first.len_utf8()..];
    if rest.chars().any(char::is_uppercase)
        || word == "I"
        || ["I'm", "I'll", "I've", "I'd", "I’m", "I’ll", "I’ve", "I’d"].contains(&word)
    {
        return text.to_owned();
    }
    format!(
        "{}{}{}",
        &text[..start],
        first.to_lowercase(),
        &text[start + first.len_utf8()..]
    )
}

fn trim_transcript_boundary(text: &str) -> &str {
    // Invisible boundary markers must not shield adjacent whitespace from trim.
    text.trim_matches(|character: char| {
        character.is_whitespace() || matches!(character, '\u{feff}' | '\u{200b}')
    })
}

fn capitalize_dictation_start(text: &str) -> String {
    let start = sentence_content_start(text);
    let token = text[start..].split_whitespace().next().unwrap_or_default();
    // Preserve literal URLs, email addresses, paths and identifiers.
    if token.contains(['/', '\\', '@', '_'])
        || token
            .trim_end_matches(['.', '!', '?', ',', ';', ':'])
            .contains('.')
    {
        return text.to_owned();
    }
    capitalize_sentence_start(text)
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
        .split(|c: char| !c.is_alphabetic() && !matches!(c, '\'' | '’' | '-'))
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
        while result.ends_with([' ', '\t', '.', ',', ';', ':', '!', '?']) {
            result.pop();
        }
        result.push(punctuation);

        // Like line-break cues, consume ASR punctuation attached to the cue.
        // Retain trailing whitespace: punctuation alone must not join words.
        let mut scan = end;
        let mut punctuation_end = end;
        while let Some(c) = text[scan..].chars().next() {
            if !matches!(c, ' ' | '\t' | '.' | ',' | ';' | ':' | '!' | '?') {
                break;
            }
            scan += c.len_utf8();
            if !matches!(c, ' ' | '\t') {
                punctuation_end = scan;
            }
        }
        cursor = punctuation_end;
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
    fn cursor_context_distinguishes_continuations_from_sentence_and_paragraph_boundaries() {
        for prefix in [
            "typed some text",
            "typed some text ",
            "text,\t",
            "unfinished (\"",
            "quoted text’ ",
        ] {
            assert!(
                InsertionContext::from_prefix(prefix).continues_sentence,
                "{prefix:?}"
            );
        }
        for prefix in [
            "",
            " \t",
            "finished. ",
            "finished!\" ",
            "finished? (",
            "new paragraph\n\t",
            "finished… ",
        ] {
            assert!(
                !InsertionContext::from_prefix(prefix).continues_sentence,
                "{prefix:?}"
            );
        }
    }

    #[test]
    fn continuation_casing_adjusts_only_the_initial_word_and_keeps_later_sentences() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (raw, expected) in [
            (
                "And this continues. Another sentence.",
                "and this continues. Another sentence.",
            ),
            ("\"And this continues\"", "\"and this continues\""),
            ("Également ici", "également ici"),
            ("Well-known names", "well-known names"),
            ("I agree", "I agree"),
            ("I'll finish", "I'll finish"),
            ("I’m ready", "I’m ready"),
            ("NASA is ready", "NASA is ready"),
            ("OpenAI is ready", "OpenAI is ready"),
            ("eBay is ready", "eBay is ready"),
            ("Next period another sentence", "next. Another sentence"),
            ("new line Next item", "Next item"),
        ] {
            assert_eq!(
                process_transcription_text_with_context(
                    raw,
                    Some(InsertionContext::from_prefix("typed text ")),
                    &[],
                    options
                ),
                expected,
                "{raw}"
            );
        }
        assert_eq!(
            process_transcription_text_with_context(
                "And this continues",
                Some(InsertionContext::from_prefix("typed text")),
                &[],
                options
            ),
            " and this continues"
        );
        for context in [
            None,
            Some(InsertionContext::from_prefix("Done. ")),
            Some(InsertionContext::from_prefix("\n")),
        ] {
            assert_eq!(
                process_transcription_text_with_context("And this begins", context, &[], options),
                "And this begins"
            );
        }
        assert_eq!(
            process_transcription_text_with_context(
                "And literal text",
                Some(InsertionContext::from_prefix("typed text ")),
                &[],
                TextProcessingOptions {
                    formatting_commands_enabled: false,
                    ..options
                }
            ),
            "And literal text"
        );
    }

    #[test]
    fn continuation_casing_respects_personalized_names_corrections_and_snippets() {
        let context = Some(InsertionContext::from_prefix("send this to "));
        let words = [crate::personalization::WordEntry {
            phrase: "John Smith".to_owned(),
            enabled: true,
            boost: 1.0,
        }];
        let corrections = [CorrectionEntry {
            source: "And".to_owned(),
            replacement: "AND".to_owned(),
            enabled: true,
            case_sensitive: true,
        }];
        let snippets = [crate::personalization::SnippetEntry {
            trigger: "Signature".to_owned(),
            expansion: "Kind regards,\nJohn".to_owned(),
            enabled: true,
            case_sensitive: true,
        }];
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &corrections,
            snippets: &snippets,
        };
        assert_eq!(
            process_transcription_text_with_context(
                "John Smith tomorrow",
                context,
                &words,
                options
            ),
            "John Smith tomorrow"
        );
        assert_eq!(
            process_transcription_text_with_context("And this continues", context, &words, options),
            "AND this continues"
        );
        assert_eq!(
            process_transcription_text_with_context("Signature", context, &words, options),
            "Kind regards,\nJohn"
        );
        assert_eq!(
            process_transcription_text_with_context("Ordinary text", context, &words, options),
            "ordinary text"
        );
    }

    #[test]
    fn push_to_talk_requires_key_up_to_stop() {
        let mut controller = ActivationController::new(ActivationMode::PushToTalk);
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Released),
            ActivationAction::Stop
        );
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Released),
            ActivationAction::Ignore
        );
    }

    #[test]
    fn initiating_binding_owns_mixed_mode_recordings() {
        let hold = ActivationMode::PushToTalk;
        let toggle = ActivationMode::Toggle;
        let mut controller = ActivationController::new(hold);
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(controller.active_mode(), Some(toggle));
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Pressed),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("Ctrl+Alt+Space", hold, ShortcutEdge::Pressed),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("Ctrl+Alt+Space", hold, ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("F20", hold, ShortcutEdge::Pressed),
            ActivationAction::Stop
        );
        controller.reset();
        assert_eq!(
            controller.handle_binding_with_mode("Ctrl+Alt+Space", hold, ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Pressed),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding_with_mode("Ctrl+Alt+Space", toggle, ShortcutEdge::Released),
            ActivationAction::Stop
        );
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        controller.reset();
        assert_eq!(
            controller.handle_binding_with_mode("F20", toggle, ShortcutEdge::Released),
            ActivationAction::Ignore
        );
    }

    #[test]
    fn toggle_changes_on_each_press_and_ignores_release() {
        let mut controller = ActivationController::new(ActivationMode::Toggle);
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            controller.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Stop
        );
    }

    #[test]
    fn activation_mode_changes_do_not_change_the_active_gesture() {
        let mut push_to_talk = ActivationController::new(ActivationMode::PushToTalk);
        assert_eq!(
            push_to_talk.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        push_to_talk.set_mode(ActivationMode::Toggle);
        assert_eq!(
            push_to_talk.handle_binding("test", ShortcutEdge::Released),
            ActivationAction::Stop
        );

        let mut toggle = ActivationController::new(ActivationMode::Toggle);
        assert_eq!(
            toggle.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Start
        );
        toggle.set_mode(ActivationMode::PushToTalk);
        assert_eq!(
            toggle.handle_binding("test", ShortcutEdge::Released),
            ActivationAction::Ignore
        );
        assert_eq!(
            toggle.handle_binding("test", ShortcutEdge::Pressed),
            ActivationAction::Stop
        );
        toggle.reset();
        assert_eq!(
            toggle.handle_binding("test", ShortcutEdge::Pressed),
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
    fn new_dictations_start_cleanly_while_known_continuations_keep_their_case() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: false,
            corrections: &[],
            snippets: &[],
        };
        for raw in [
            " looks good",
            "\u{feff} looks good",
            "\u{200b} looks good ",
            "\tlooks good \u{200b}\u{feff}\r\n",
        ] {
            assert_eq!(process_transcription_text(raw, None, options), "Looks good");
        }
        assert_eq!(process_transcription_text("élan", None, options), "Élan");
        assert_eq!(
            process_transcription_text("“looks good”", None, options),
            "“Looks good”"
        );
        assert_eq!(
            process_transcription_text_with_context(
                "looks good",
                Some(InsertionContext::from_prefix("a")),
                &[],
                options
            ),
            " looks good"
        );
        assert_eq!(
            process_transcription_text_with_context(
                "looks good",
                Some(InsertionContext::from_prefix("existing text ")),
                &[],
                options
            ),
            "looks good"
        );
        assert_eq!(
            process_transcription_text("looks good", Some('\n'), options),
            "Looks good"
        );
        assert_eq!(
            process_transcription_text("123 items", None, options),
            "123 items"
        );
        assert_eq!(
            process_transcription_text("\u{200b} \r\n", None, options),
            ""
        );
        for prefix in ["", " ", "Done. ", "Done! ", "\n"] {
            assert_eq!(
                process_transcription_text_with_context(
                    " looks good ",
                    Some(InsertionContext::from_prefix(prefix)),
                    &[],
                    options
                ),
                "Looks good"
            );
        }
        assert_eq!(
            process_transcription_text(
                " looks good ",
                None,
                TextProcessingOptions {
                    formatting_commands_enabled: false,
                    ..options
                }
            ),
            "looks good"
        );
        for raw in [
            "eBay works",
            "iPhone works",
            "d’Artagnan agrees",
            "NASA works",
            "https://example.com/path",
            "person@example.com",
            "example.com works",
            "some_identifier works",
            "c:\\some\\path",
        ] {
            assert_eq!(process_transcription_text(raw, None, options), raw);
        }
        assert_eq!(
            process_transcription_text("hello\u{200b} world", None, options),
            "Hello\u{200b} world"
        );
    }

    #[test]
    fn new_dictation_preserves_explicit_personalization_casing() {
        let corrections = [CorrectionEntry {
            source: "brand".to_owned(),
            replacement: "iPhone".to_owned(),
            enabled: true,
            case_sensitive: true,
        }];
        let snippets = [crate::personalization::SnippetEntry {
            trigger: "signature".to_owned(),
            expansion: "best regards".to_owned(),
            enabled: true,
            case_sensitive: true,
        }];
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: false,
            corrections: &corrections,
            snippets: &snippets,
        };
        assert_eq!(
            process_transcription_text("brand works", None, options),
            "iPhone works"
        );
        assert_eq!(
            process_transcription_text("signature", None, options),
            "best regards"
        );
        let words = [crate::personalization::WordEntry {
            phrase: "iReady".to_owned(),
            enabled: true,
            boost: 2.5,
        }];
        assert_eq!(
            process_transcription_text_with_context("iready works", None, &words, options),
            "iready works"
        );
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
            "Hello, world."
        );
        assert_eq!(
            process_transcription_text("new paragraph next new line item", None, options),
            "Next\nitem"
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
            "\"Hello\""
        );
        assert_eq!(
            process_transcription_text("commawheel", None, options),
            "Commawheel"
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
                "Hello. \"Welcome\".",
            ),
            ("hello period new paragraph élève", "Hello.\n\nÉlève"),
            ("hello period iPhone and eBay", "Hello. iPhone and eBay"),
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
            "Already punctuated. next stays as recognized.",
        ] {
            assert_eq!(process_transcription_text(raw, None, options), raw);
        }
        assert_eq!(
            process_transcription_text("new line item", None, options),
            "Item"
        );
        assert_eq!(
            process_transcription_text("notes new line item", None, options),
            "Notes\nitem"
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
            "Hello. eBay"
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
        assert_eq!(text, "Hello. 12.5 kilograms");
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
                "Hello. d’Artagnan arrived",
            ),
            (
                "hello period d'Artagnan arrived",
                "Hello. d'Artagnan arrived",
            ),
            (
                "Send this scratch that d’Artagnan arrived",
                "d’Artagnan arrived",
            ),
            ("hello period l’esprit", "Hello. L’esprit"),
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
    fn sentence_casing_preserves_hyphenated_mixed_case_names() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (source, expected) in [
            ("hello period i-Ready is open", "Hello. i-Ready is open"),
            ("Send this scratch that i-Ready is open", "i-Ready is open"),
            ("hello period well-known names", "Hello. Well-known names"),
        ] {
            assert_eq!(
                process_transcription_text(source, None, options),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn spoken_punctuation_owns_interleaved_horizontal_marks() {
        let options = TextProcessingOptions {
            formatting_commands_enabled: true,
            self_correction_enabled: true,
            corrections: &[],
            snippets: &[],
        };
        for (source, expected) in [
            ("Hello. , period next", "Hello. Next"),
            ("Hello period. , next", "Hello. Next"),
            ("Hello.\t, period.\t, next", "Hello. Next"),
            ("Hello! ; comma next", "Hello, next"),
            ("Hello... next", "Hello... next"),
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

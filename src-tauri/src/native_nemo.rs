use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    fs,
    path::{Path, PathBuf},
    ptr,
    sync::{Arc, Mutex},
};

use libloading::{Library, Symbol};
use tracing::warn;

use crate::{
    domain::{BackendError, LanguagePreference, TranscriptionRequest},
    streaming::{FinalTranscript, PartialTranscript, StreamingRequest, StreamingResult},
};

type RecognizerOpaque = c_void;
type StreamOpaque = c_void;
type ResultOpaque = c_void;
type Status = c_int;

const STATUS_OK: Status = 0;
const STATUS_CANCELLED: Status = 4;

#[repr(C)]
struct BackendConfig {
    size: usize,
    gpu: i32,
}

#[repr(C)]
struct ModelConfig {
    size: usize,
    path: *const c_char,
    name: *const c_char,
}

#[repr(C)]
struct RecognizerConfig {
    size: usize,
    backend: *const BackendConfig,
    model: *const ModelConfig,
    streaming: *const c_void,
    decoder: *const c_void,
    vad: *const c_void,
    endpointing: *const c_void,
    postproc: *const c_void,
    diar: *const c_void,
    batching: *const c_void,
}

#[repr(C)]
struct SpeechContext {
    size: usize,
    phrases: *const *const c_char,
    phrase_count: usize,
    boost: f32,
}

#[repr(C)]
struct RecognitionOptions {
    size: usize,
    request_id: *const c_char,
    language_code: *const c_char,
    interim_results: bool,
    enable_word_time_offsets: bool,
    enable_automatic_punctuation: bool,
    verbatim_transcripts: bool,
    profanity_filter: bool,
    stop_history_eou_ms: i32,
    speech_contexts: *const SpeechContext,
    speech_context_count: usize,
    max_alternatives: i32,
    enable_speaker_diarization: bool,
    max_speaker_count: i32,
}

type CreateFn = unsafe extern "C" fn(*const RecognizerConfig, *mut *mut RecognizerOpaque) -> Status;
type DestroyFn = unsafe extern "C" fn(*mut RecognizerOpaque);
type OptionsDefaultFn = unsafe extern "C" fn() -> RecognitionOptions;
type RecognizeF32Fn = unsafe extern "C" fn(
    *mut RecognizerOpaque,
    *const RecognitionOptions,
    *const f32,
    usize,
    i32,
    *mut *mut ResultOpaque,
) -> Status;
type ResultTranscriptFn = unsafe extern "C" fn(*const ResultOpaque, usize) -> *const c_char;
type ResultDestroyFn = unsafe extern "C" fn(*mut ResultOpaque);
type StreamingRecognizeFn = unsafe extern "C" fn(
    *mut RecognizerOpaque,
    *const RecognitionOptions,
    *mut *mut StreamOpaque,
) -> Status;
type StreamPushF32Fn = unsafe extern "C" fn(*mut StreamOpaque, *const f32, usize, i32) -> Status;
type StreamForceEndpointFn = unsafe extern "C" fn(*mut StreamOpaque) -> Status;
type StreamFinishFn = unsafe extern "C" fn(*mut StreamOpaque) -> Status;
type StreamNextFn = unsafe extern "C" fn(*mut StreamOpaque, *mut *mut ResultOpaque) -> Status;
type StreamCloseFn = unsafe extern "C" fn(*mut StreamOpaque);
type ResultIsFinalFn = unsafe extern "C" fn(*const ResultOpaque) -> bool;
type ResultAudioProcessedFn = unsafe extern "C" fn(*const ResultOpaque) -> f32;
type LastErrorFn = unsafe extern "C" fn() -> *const c_char;
type VersionFn = unsafe extern "C" fn() -> *const c_char;

#[derive(Clone, Copy)]
struct StreamingApi {
    recognize: StreamingRecognizeFn,
    push_f32: StreamPushF32Fn,
    force_endpoint: StreamForceEndpointFn,
    finish: StreamFinishFn,
    next: StreamNextFn,
    close: StreamCloseFn,
    result_is_final: ResultIsFinalFn,
    audio_processed: Option<ResultAudioProcessedFn>,
}

#[derive(Default)]
struct OptionalStreamingSymbols {
    recognize: Option<StreamingRecognizeFn>,
    push_f32: Option<StreamPushF32Fn>,
    force_endpoint: Option<StreamForceEndpointFn>,
    finish: Option<StreamFinishFn>,
    next: Option<StreamNextFn>,
    close: Option<StreamCloseFn>,
    result_is_final: Option<ResultIsFinalFn>,
}

impl OptionalStreamingSymbols {
    fn into_api(self, audio_processed: Option<ResultAudioProcessedFn>) -> Option<StreamingApi> {
        Some(StreamingApi {
            recognize: self.recognize?,
            push_f32: self.push_f32?,
            force_endpoint: self.force_endpoint?,
            finish: self.finish?,
            next: self.next?,
            close: self.close?,
            result_is_final: self.result_is_final?,
            audio_processed,
        })
    }
}

struct NativeApi {
    _library: Library,
    create: CreateFn,
    destroy: DestroyFn,
    options_default: OptionsDefaultFn,
    recognize_f32: RecognizeF32Fn,
    result_transcript: ResultTranscriptFn,
    result_destroy: ResultDestroyFn,
    streaming: Option<StreamingApi>,
    last_error: LastErrorFn,
    version: VersionFn,
}

// The API is a stable C ABI and all recognizer calls are serialized by the
// backend mutex. The dynamic library itself is kept alive by NativeApi.
unsafe impl Send for NativeApi {}
unsafe impl Sync for NativeApi {}

struct RecognizerOwner {
    api: Arc<NativeApi>,
    handle: *mut RecognizerOpaque,
    /// The C ABI recognizer is treated as single-threaded by this boundary.
    /// A streaming session holds the same owner, so it also keeps the
    /// recognizer alive until its stream has been closed.
    call_lock: Mutex<()>,
}

unsafe impl Send for RecognizerOwner {}
unsafe impl Sync for RecognizerOwner {}

impl Drop for RecognizerOwner {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.api.destroy)(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}

#[derive(Clone)]
pub struct NativeNemoRecognizer {
    owner: Arc<RecognizerOwner>,
}

impl NativeNemoRecognizer {
    pub fn load(
        runtime_path: &Path,
        model_path: &Path,
        gpu_index: i32,
    ) -> Result<Self, BackendError> {
        let library_path = find_library(runtime_path).ok_or_else(|| {
            BackendError::MissingRuntimeModel(
                "no NeMo-Speech.cpp DLL found in the verified runtime".to_owned(),
            )
        })?;
        let api = Arc::new(load_api(&library_path)?);
        let model_path = CString::new(model_path.to_string_lossy().as_bytes()).map_err(|_| {
            BackendError::MissingRuntimeModel("model path contains an invalid character".to_owned())
        })?;
        let backend = BackendConfig {
            size: std::mem::size_of::<BackendConfig>(),
            gpu: gpu_index,
        };
        let model = ModelConfig {
            size: std::mem::size_of::<ModelConfig>(),
            path: model_path.as_ptr(),
            name: ptr::null(),
        };
        let config = RecognizerConfig {
            size: std::mem::size_of::<RecognizerConfig>(),
            backend: &backend,
            model: &model,
            streaming: ptr::null(),
            decoder: ptr::null(),
            vad: ptr::null(),
            endpointing: ptr::null(),
            postproc: ptr::null(),
            diar: ptr::null(),
            batching: ptr::null(),
        };
        let mut handle = ptr::null_mut();
        let status = unsafe { (api.create)(&config, &mut handle) };
        if status != STATUS_OK || handle.is_null() {
            let detail = last_error(
                &api,
                format!("NeMo recognizer creation failed with status {status}"),
            );
            if !handle.is_null() {
                unsafe { (api.destroy)(handle) };
            }
            return Err(BackendError::MissingRuntimeModel(detail));
        }
        Ok(Self {
            owner: Arc::new(RecognizerOwner {
                api,
                handle,
                call_lock: Mutex::new(()),
            }),
        })
    }

    pub fn version(&self) -> Option<String> {
        let _guard = self.owner.call_lock.lock().ok()?;
        let value = unsafe { (self.owner.api.version)() };
        (!value.is_null()).then(|| unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() })
    }

    pub fn streaming_available(&self) -> bool {
        self.owner.api.streaming.is_some()
    }

    pub fn start_stream(
        &self,
        request: StreamingRequest,
    ) -> Result<NativeNemoStreamingSession, BackendError> {
        NativeNemoStreamingSession::start(Arc::clone(&self.owner), request)
    }

    pub fn recognize(&self, request: &TranscriptionRequest) -> Result<String, BackendError> {
        if request.audio_samples.is_empty() {
            return Err(BackendError::EmptyRecording);
        }
        if !(8_000..=96_000).contains(&request.sample_rate) {
            return Err(BackendError::Unsupported(format!(
                "captured audio uses unsupported sample rate {} Hz",
                request.sample_rate
            )));
        }
        if request
            .audio_samples
            .iter()
            .any(|sample| !sample.is_finite())
        {
            return Err(BackendError::LocalRecognitionFailure(
                "captured audio contains a non-finite sample".to_owned(),
            ));
        }
        let request_id = CString::new(request.request_id.as_str()).map_err(|_| {
            BackendError::Unsupported("request id contains an invalid character".to_owned())
        })?;
        let _guard =
            self.owner.call_lock.lock().map_err(|_| {
                BackendError::Unsupported("native recognizer lock poisoned".to_owned())
            })?;
        let mut options = unsafe { (self.owner.api.options_default)() };
        options.size = std::mem::size_of::<RecognitionOptions>();
        options.request_id = request_id.as_ptr();
        options.language_code = language_code(request.language);
        options.enable_automatic_punctuation = true;
        let mut result = ptr::null_mut();
        let status = unsafe {
            (self.owner.api.recognize_f32)(
                self.owner.handle,
                &options,
                request.audio_samples.as_ptr(),
                request.audio_samples.len(),
                request.sample_rate as i32,
                &mut result,
            )
        };
        if status != STATUS_OK || result.is_null() {
            let detail = last_error(&self.owner.api, String::new());
            let detail = if detail.trim().is_empty() {
                format!(
                    "NeMo recognition failed with status {status} for {} samples at {} Hz",
                    request.audio_samples.len(),
                    request.sample_rate
                )
            } else {
                detail
            };
            warn!(
                status,
                sample_count = request.audio_samples.len(),
                sample_rate = request.sample_rate,
                error = %detail,
                "native local recognition failed"
            );
            return Err(BackendError::LocalRecognitionFailure(detail));
        }
        let transcript = unsafe { (self.owner.api.result_transcript)(result, 0) };
        let text = if transcript.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(transcript).to_string_lossy().into_owned() }
        };
        unsafe {
            (self.owner.api.result_destroy)(result);
        }
        if text.trim().is_empty() {
            return Err(BackendError::EmptyRecording);
        }
        Ok(text)
    }
}

/// Owns every allocation referenced by `RecognitionOptions` for the entire
/// native stream lifetime.  The native implementation may retain those
/// pointers after `streaming_recognize` returns, so keeping only temporary
/// CStrings here would make context phrases use-after-free.
struct StreamingOptionsStorage {
    _request_id: CString,
    _language_code: Option<CString>,
    _phrase_values: Vec<CString>,
    _phrase_ptrs: Vec<*const c_char>,
    _speech_context: Option<Box<SpeechContext>>,
    options: Box<RecognitionOptions>,
}

impl StreamingOptionsStorage {
    fn new(api: &NativeApi, request: &StreamingRequest) -> Result<Self, BackendError> {
        let options = unsafe { (api.options_default)() };
        Self::from_options(options, request)
    }

    fn from_options(
        mut options: RecognitionOptions,
        request: &StreamingRequest,
    ) -> Result<Self, BackendError> {
        let request_id = CString::new(request.request_id.as_str()).map_err(|_| {
            BackendError::Unsupported("stream request id contains an invalid character".to_owned())
        })?;
        let language_code = language_code_value(request.language)
            .map(CString::new)
            .transpose()
            .map_err(|_| {
                BackendError::Unsupported(
                    "stream language preference contains an invalid character".to_owned(),
                )
            })?;
        let phrase_values = request
            .context_phrases
            .iter()
            .map(|phrase| {
                CString::new(phrase.as_str()).map_err(|_| {
                    BackendError::Unsupported(
                        "stream context phrase contains an invalid character".to_owned(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let phrase_ptrs = phrase_values
            .iter()
            .map(|phrase| phrase.as_ptr())
            .collect::<Vec<_>>();
        let speech_context = (!phrase_ptrs.is_empty()).then(|| {
            Box::new(SpeechContext {
                size: std::mem::size_of::<SpeechContext>(),
                phrases: phrase_ptrs.as_ptr(),
                phrase_count: phrase_ptrs.len(),
                // Zero asks the runtime for its bounded default boost.
                boost: 0.0,
            })
        });
        options.size = std::mem::size_of::<RecognitionOptions>();
        options.request_id = request_id.as_ptr();
        options.language_code = language_code
            .as_ref()
            .map(|value| value.as_ptr())
            .unwrap_or(ptr::null());
        options.interim_results = request.interim_results;
        options.enable_automatic_punctuation = true;
        options.speech_contexts = speech_context
            .as_deref()
            .map(|context| context as *const SpeechContext)
            .unwrap_or(ptr::null());
        options.speech_context_count = usize::from(speech_context.is_some());
        options.max_alternatives = 1;
        options.enable_speaker_diarization = false;
        options.max_speaker_count = 0;
        Ok(Self {
            _request_id: request_id,
            _language_code: language_code,
            _phrase_values: phrase_values,
            _phrase_ptrs: phrase_ptrs,
            _speech_context: speech_context,
            options: Box::new(options),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingSessionState {
    Open,
    Finishing,
    Closed,
    Cancelled,
}

/// A safe owner for one NeMo streaming recognition handle.
///
/// Calls require `&mut self`, are serialized with the recognizer's lock, and
/// always close the native stream on drop.  The shared recognizer owner keeps
/// the recognizer and dynamic library alive until this session is gone.
pub struct NativeNemoStreamingSession {
    owner: Arc<RecognizerOwner>,
    stream: *mut StreamOpaque,
    _options: StreamingOptionsStorage,
    state: StreamingSessionState,
}

unsafe impl Send for NativeNemoStreamingSession {}

impl NativeNemoStreamingSession {
    fn start(owner: Arc<RecognizerOwner>, request: StreamingRequest) -> Result<Self, BackendError> {
        let streaming = owner.api.streaming.ok_or_else(|| {
            BackendError::Unsupported(
                "the installed NeMo runtime does not expose streaming recognition".to_owned(),
            )
        })?;
        let _guard = owner
            .call_lock
            .lock()
            .map_err(|_| BackendError::Unsupported("native recognizer lock poisoned".to_owned()))?;
        let options = StreamingOptionsStorage::new(&owner.api, &request)?;
        let mut stream = ptr::null_mut();
        let status = unsafe { (streaming.recognize)(owner.handle, &*options.options, &mut stream) };
        if status != STATUS_OK || stream.is_null() {
            if !stream.is_null() {
                unsafe { (streaming.close)(stream) };
            }
            drop(_guard);
            return Err(streaming_failure(
                &owner.api,
                status,
                "could not start NeMo streaming recognition",
            ));
        }
        drop(_guard);
        Ok(Self {
            owner,
            stream,
            _options: options,
            state: StreamingSessionState::Open,
        })
    }

    pub fn state(&self) -> StreamingSessionState {
        self.state
    }

    pub fn push_f32(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), BackendError> {
        self.ensure_pushable()?;
        if samples.is_empty() {
            return Ok(());
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(BackendError::LocalRecognitionFailure(
                "stream audio contains a non-finite sample".to_owned(),
            ));
        }
        if sample_rate != 0 && !(8_000..=96_000).contains(&sample_rate) {
            return Err(BackendError::Unsupported(format!(
                "stream audio uses unsupported sample rate {sample_rate} Hz"
            )));
        }
        let streaming = self.streaming_api()?;
        let _guard = self.lock_owner()?;
        let status = unsafe {
            (streaming.push_f32)(
                self.stream,
                samples.as_ptr(),
                samples.len(),
                sample_rate as i32,
            )
        };
        if status == STATUS_OK {
            Ok(())
        } else {
            Err(streaming_failure(
                &self.owner.api,
                status,
                "could not push audio into NeMo stream",
            ))
        }
    }

    pub fn force_endpoint(&mut self) -> Result<(), BackendError> {
        self.ensure_pushable()?;
        let streaming = self.streaming_api()?;
        let _guard = self.lock_owner()?;
        let status = unsafe { (streaming.force_endpoint)(self.stream) };
        if status == STATUS_OK {
            Ok(())
        } else {
            Err(streaming_failure(
                &self.owner.api,
                status,
                "could not force the NeMo stream endpoint",
            ))
        }
    }

    pub fn finish(&mut self) -> Result<(), BackendError> {
        if self.state != StreamingSessionState::Open {
            return match self.state {
                StreamingSessionState::Cancelled => Err(BackendError::Cancellation),
                StreamingSessionState::Finishing => Ok(()),
                StreamingSessionState::Closed => Err(BackendError::Unsupported(
                    "NeMo stream is already closed".to_owned(),
                )),
                StreamingSessionState::Open => unreachable!(),
            };
        }
        let streaming = self.streaming_api()?;
        let status = {
            let _guard = self.lock_owner()?;
            unsafe { (streaming.finish)(self.stream) }
        };
        if status == STATUS_OK {
            self.state = StreamingSessionState::Finishing;
            Ok(())
        } else {
            Err(streaming_failure(
                &self.owner.api,
                status,
                "could not finish the NeMo stream",
            ))
        }
    }

    /// Pull all currently available results after finish.  `stream_next`
    /// returns `OK` with a null result when its queue is empty; that is the
    /// drain boundary.  A generous cap protects the worker from a broken
    /// runtime that never reaches that boundary.
    pub fn drain(&mut self) -> Result<Vec<StreamingResult>, BackendError> {
        if self.state == StreamingSessionState::Open {
            self.finish()?;
        }
        let mut results = Vec::new();
        for _ in 0..1_024 {
            match self.next()? {
                Some(result) => results.push(result),
                None => return Ok(results),
            }
        }
        Err(BackendError::LocalRecognitionFailure(
            "NeMo stream produced too many results while draining".to_owned(),
        ))
    }

    /// Finish and return only the first final result, preserving Lyra's
    /// one-gesture/one-final contract while still draining all pending native
    /// handles.
    pub fn drain_final(&mut self) -> Result<Option<FinalTranscript>, BackendError> {
        let results = self.drain()?;
        let mut final_result = None;
        for result in results {
            if let StreamingResult::Final(result) = result {
                if final_result.is_none() {
                    final_result = Some(result);
                }
            }
        }
        Ok(final_result)
    }

    pub fn next(&mut self) -> Result<Option<StreamingResult>, BackendError> {
        if matches!(
            self.state,
            StreamingSessionState::Open | StreamingSessionState::Finishing
        ) {
            // allowed
        } else if self.state == StreamingSessionState::Cancelled {
            return Err(BackendError::Cancellation);
        } else {
            return Err(BackendError::Unsupported(
                "NeMo stream is already closed".to_owned(),
            ));
        }
        let streaming = self.streaming_api()?;
        let _guard = self.lock_owner()?;
        let mut result = ptr::null_mut();
        let status = unsafe { (streaming.next)(self.stream, &mut result) };
        if status != STATUS_OK {
            return Err(streaming_failure(
                &self.owner.api,
                status,
                "could not pull a result from the NeMo stream",
            ));
        }
        if result.is_null() {
            return Ok(None);
        }
        let is_final = unsafe { (streaming.result_is_final)(result) };
        let text = unsafe { (self.owner.api.result_transcript)(result, 0) };
        let text = if text.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() }
        };
        let audio_processed_ms = streaming
            .audio_processed
            .map(|accessor| unsafe { accessor(result) })
            .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
            .map(|seconds| (f64::from(seconds) * 1_000.0).round() as u64);
        unsafe {
            (self.owner.api.result_destroy)(result);
        }
        if is_final {
            Ok(Some(StreamingResult::Final(FinalTranscript {
                text,
                audio_processed_ms,
            })))
        } else {
            Ok(Some(StreamingResult::Partial(PartialTranscript {
                text,
                audio_processed_ms,
            })))
        }
    }

    pub fn close(&mut self) {
        if self.stream.is_null() {
            if self.state != StreamingSessionState::Cancelled {
                self.state = StreamingSessionState::Closed;
            }
            return;
        }
        if let Some(streaming) = self.owner.api.streaming {
            if let Ok(_guard) = self.owner.call_lock.lock() {
                unsafe { (streaming.close)(self.stream) };
            }
        }
        self.stream = ptr::null_mut();
        self.state = StreamingSessionState::Closed;
    }

    pub fn cancel(&mut self) {
        if self.stream.is_null() {
            self.state = StreamingSessionState::Cancelled;
            return;
        }
        if let Some(streaming) = self.owner.api.streaming {
            if let Ok(_guard) = self.owner.call_lock.lock() {
                unsafe { (streaming.close)(self.stream) };
            }
        }
        self.stream = ptr::null_mut();
        self.state = StreamingSessionState::Cancelled;
    }

    fn ensure_pushable(&self) -> Result<(), BackendError> {
        match self.state {
            StreamingSessionState::Open => Ok(()),
            StreamingSessionState::Finishing => Err(BackendError::Unsupported(
                "cannot push audio after finishing the NeMo stream".to_owned(),
            )),
            StreamingSessionState::Cancelled => Err(BackendError::Cancellation),
            StreamingSessionState::Closed => Err(BackendError::Unsupported(
                "NeMo stream is already closed".to_owned(),
            )),
        }
    }

    fn streaming_api(&self) -> Result<StreamingApi, BackendError> {
        self.owner.api.streaming.ok_or_else(|| {
            BackendError::Unsupported(
                "the installed NeMo runtime does not expose streaming recognition".to_owned(),
            )
        })
    }

    fn lock_owner(&self) -> Result<std::sync::MutexGuard<'_, ()>, BackendError> {
        self.owner
            .call_lock
            .lock()
            .map_err(|_| BackendError::Unsupported("native recognizer lock poisoned".to_owned()))
    }
}

impl Drop for NativeNemoStreamingSession {
    fn drop(&mut self) {
        self.close();
    }
}

fn streaming_failure(api: &NativeApi, status: Status, fallback: &str) -> BackendError {
    if status == STATUS_CANCELLED {
        return BackendError::Cancellation;
    }
    let detail = last_error(api, String::new());
    let detail = if detail.trim().is_empty() {
        format!("{fallback} (status {status})")
    } else {
        detail
    };
    BackendError::LocalRecognitionFailure(detail)
}

fn load_api(path: &Path) -> Result<NativeApi, BackendError> {
    let library = load_library(path).map_err(|error| {
        BackendError::MissingRuntimeModel(format!("could not load {}: {error:?}", path.display()))
    })?;
    unsafe {
        let symbols = OptionalStreamingSymbols {
            recognize: optional_symbol(&library, b"nemo_speech_asr_streaming_recognize\0")
                .map(|symbol| *symbol),
            push_f32: optional_symbol(&library, b"nemo_speech_asr_stream_push_f32\0")
                .map(|symbol| *symbol),
            force_endpoint: optional_symbol(&library, b"nemo_speech_asr_stream_force_endpoint\0")
                .map(|symbol| *symbol),
            finish: optional_symbol(&library, b"nemo_speech_asr_stream_finish\0")
                .map(|symbol| *symbol),
            next: optional_symbol(&library, b"nemo_speech_asr_stream_next\0").map(|symbol| *symbol),
            close: optional_symbol(&library, b"nemo_speech_asr_stream_close\0")
                .map(|symbol| *symbol),
            result_is_final: optional_symbol(&library, b"nemo_speech_asr_result_is_final\0")
                .map(|symbol| *symbol),
        };
        let audio_processed =
            optional_symbol(&library, b"nemo_speech_asr_result_audio_processed\0")
                .map(|symbol| *symbol);
        let streaming = symbols.into_api(audio_processed);
        Ok(NativeApi {
            create: *symbol(&library, b"nemo_speech_asr_create\0")?,
            destroy: *symbol(&library, b"nemo_speech_asr_destroy\0")?,
            options_default: *symbol(&library, b"nemo_speech_asr_recognition_options_default\0")?,
            recognize_f32: *symbol(&library, b"nemo_speech_asr_recognize_f32\0")?,
            result_transcript: *symbol(&library, b"nemo_speech_asr_result_transcript\0")?,
            result_destroy: *symbol(&library, b"nemo_speech_asr_result_destroy\0")?,
            streaming,
            last_error: *symbol(&library, b"nemo_speech_asr_last_error\0")?,
            version: *symbol(&library, b"nemo_speech_asr_version\0")?,
            _library: library,
        })
    }
}

#[cfg(windows)]
fn load_library(path: &Path) -> Result<Library, libloading::Error> {
    use libloading::os::windows::{
        Library as WindowsLibrary, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    };

    // The NeMo archive is self-contained under its runtime `bin` directory.
    // LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR makes Windows resolve transitive DLL
    // dependencies (ggml, CUDA, and the MSVC runtime) beside the selected
    // library instead of depending on the process working directory or PATH.
    unsafe {
        WindowsLibrary::load_with_flags(
            path,
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        )
        .map(Into::into)
    }
}

#[cfg(not(windows))]
fn load_library(path: &Path) -> Result<Library, libloading::Error> {
    unsafe { Library::new(path) }
}

unsafe fn symbol<'a, T>(library: &'a Library, name: &[u8]) -> Result<Symbol<'a, T>, BackendError> {
    library.get(name).map_err(|error| {
        BackendError::MissingRuntimeModel(format!(
            "NeMo runtime is missing symbol {}: {error}",
            String::from_utf8_lossy(name)
        ))
    })
}

unsafe fn optional_symbol<'a, T>(library: &'a Library, name: &[u8]) -> Option<Symbol<'a, T>> {
    library.get(name).ok()
}

fn last_error(api: &NativeApi, fallback: String) -> String {
    let value = unsafe { (api.last_error)() };
    if value.is_null() {
        fallback
    } else {
        let message = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
        if message.trim().is_empty() {
            fallback
        } else {
            message
        }
    }
}

fn language_code(language: LanguagePreference) -> *const c_char {
    // Parakeet v3 is auto-detecting in the selected runtime. Keep the UI
    // preference visible, but do not claim that local inference can force it.
    let _ = language;
    ptr::null()
}

fn language_code_value(language: LanguagePreference) -> Option<&'static str> {
    match language {
        LanguagePreference::Auto => None,
        LanguagePreference::En => Some("en"),
        LanguagePreference::El => Some("el"),
    }
}

fn find_library(root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_library(&path) {
                return Some(found);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            // The archive includes both the C++ ABI DLL and the stable C ABI
            // DLL. Only the latter matches the function table above; loading
            // the generic ASR library first would make an installed runtime
            // appear broken even though the correct DLL is present.
            if name == "nemo_speech_asr_c.dll" {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        find_library, OptionalStreamingSymbols, RecognitionOptions, Status, StreamingOptionsStorage,
    };
    use crate::{domain::LanguagePreference, streaming::StreamingRequest};
    use std::fs;
    use std::ptr;

    unsafe extern "C" fn fake_recognize(
        _recognizer: *mut super::RecognizerOpaque,
        _options: *const super::RecognitionOptions,
        _out: *mut *mut super::StreamOpaque,
    ) -> Status {
        0
    }

    unsafe extern "C" fn fake_push(
        _stream: *mut super::StreamOpaque,
        _samples: *const f32,
        _sample_count: usize,
        _sample_rate: i32,
    ) -> Status {
        0
    }

    unsafe extern "C" fn fake_endpoint(_stream: *mut super::StreamOpaque) -> Status {
        0
    }

    unsafe extern "C" fn fake_finish(_stream: *mut super::StreamOpaque) -> Status {
        0
    }

    unsafe extern "C" fn fake_next(
        _stream: *mut super::StreamOpaque,
        _out: *mut *mut super::ResultOpaque,
    ) -> Status {
        0
    }

    unsafe extern "C" fn fake_close(_stream: *mut super::StreamOpaque) {}

    unsafe extern "C" fn fake_is_final(_result: *const super::ResultOpaque) -> bool {
        false
    }

    fn all_stream_symbols() -> OptionalStreamingSymbols {
        OptionalStreamingSymbols {
            recognize: Some(fake_recognize),
            push_f32: Some(fake_push),
            force_endpoint: Some(fake_endpoint),
            finish: Some(fake_finish),
            next: Some(fake_next),
            close: Some(fake_close),
            result_is_final: Some(fake_is_final),
        }
    }

    #[test]
    fn finds_the_stable_c_abi_library_when_cpp_dll_is_also_present() {
        let root =
            std::env::temp_dir().join(format!("lyra-native-loader-{}", uuid::Uuid::new_v4()));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create test runtime");
        let cpp = bin.join("nemo_speech_asr.dll");
        let c_abi = bin.join("nemo_speech_asr_c.dll");
        fs::write(&cpp, b"cpp").expect("write C++ ABI fixture");
        fs::write(&c_abi, b"c").expect("write C ABI fixture");

        assert_eq!(find_library(&root), Some(c_abi));
        fs::remove_dir_all(root).expect("remove test runtime");
    }

    #[test]
    fn streaming_symbols_are_optional_as_a_complete_capability() {
        assert!(all_stream_symbols().into_api(None).is_some());
        let mut missing = all_stream_symbols();
        missing.recognize = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.push_f32 = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.force_endpoint = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.finish = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.next = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.close = None;
        assert!(missing.into_api(None).is_none());
        let mut missing = all_stream_symbols();
        missing.result_is_final = None;
        assert!(missing.into_api(None).is_none());
    }

    #[test]
    fn streaming_context_phrase_storage_owns_c_strings_until_session_drop() {
        let request = StreamingRequest::new("test", LanguagePreference::Auto)
            .with_context_phrases(["Lyra", "Orion"]);
        let storage = StreamingOptionsStorage::from_options(
            RecognitionOptions {
                size: 0,
                request_id: ptr::null(),
                language_code: ptr::null(),
                interim_results: false,
                enable_word_time_offsets: false,
                enable_automatic_punctuation: false,
                verbatim_transcripts: false,
                profanity_filter: false,
                stop_history_eou_ms: 0,
                speech_contexts: ptr::null(),
                speech_context_count: 0,
                max_alternatives: 0,
                enable_speaker_diarization: false,
                max_speaker_count: 0,
            },
            &request,
        )
        .expect("build streaming options");
        assert_eq!(storage.options.speech_context_count, 1);
        let context = unsafe { &*storage.options.speech_contexts };
        assert_eq!(context.phrase_count, 2);
        let first = unsafe { std::ffi::CStr::from_ptr(*context.phrases) };
        assert_eq!(first.to_str(), Ok("Lyra"));
        assert_eq!(storage.options.interim_results, request.interim_results);
    }

    #[test]
    #[ignore = "requires the separately installed NeMo runtime and Nemotron model"]
    fn installed_nemotron_stream_smoke() {
        let runtime_path = std::env::var_os("LYRA_RUNTIME_PATH")
            .map(std::path::PathBuf::from)
            .expect("set LYRA_RUNTIME_PATH to the installed NeMo runtime");
        let model_path = std::env::var_os("LYRA_NEMOTRON_MODEL_PATH")
            .map(std::path::PathBuf::from)
            .expect("set LYRA_NEMOTRON_MODEL_PATH to the installed Nemotron model");
        let recognizer = super::NativeNemoRecognizer::load(&runtime_path, &model_path, 0)
            .expect("load installed Nemotron model");
        assert!(recognizer.streaming_available());

        let request = StreamingRequest::new("installed-smoke", LanguagePreference::En)
            .with_context_phrases(["Lyra", "Orion"]);
        let mut stream = recognizer
            .start_stream(request)
            .expect("start installed Nemotron stream");
        let silence = vec![0.0_f32; 16_000];
        stream
            .push_f32(&silence, 16_000)
            .expect("push one second of PCM");
        stream.finish().expect("finish installed Nemotron stream");
        let _results = stream.drain().expect("drain installed Nemotron stream");
        stream.close();
    }
}

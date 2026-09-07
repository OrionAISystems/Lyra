use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    fs,
    path::{Path, PathBuf},
    ptr,
    sync::Arc,
};

use libloading::{Library, Symbol};
use tracing::warn;

use crate::domain::{BackendError, LanguagePreference, TranscriptionRequest};

type RecognizerOpaque = c_void;
type ResultOpaque = c_void;
type Status = c_int;

const STATUS_OK: Status = 0;

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
    speech_contexts: *const c_void,
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
type LastErrorFn = unsafe extern "C" fn() -> *const c_char;
type VersionFn = unsafe extern "C" fn() -> *const c_char;

struct NativeApi {
    _library: Library,
    create: CreateFn,
    destroy: DestroyFn,
    options_default: OptionsDefaultFn,
    recognize_f32: RecognizeF32Fn,
    result_transcript: ResultTranscriptFn,
    result_destroy: ResultDestroyFn,
    last_error: LastErrorFn,
    version: VersionFn,
}

// The API is a stable C ABI and all recognizer calls are serialized by the
// backend mutex. The dynamic library itself is kept alive by NativeApi.
unsafe impl Send for NativeApi {}
unsafe impl Sync for NativeApi {}

pub struct NativeNemoRecognizer {
    api: Arc<NativeApi>,
    handle: *mut RecognizerOpaque,
}

unsafe impl Send for NativeNemoRecognizer {}

impl Drop for NativeNemoRecognizer {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.api.destroy)(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
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
            return Err(BackendError::MissingRuntimeModel(last_error(
                &api,
                format!("NeMo recognizer creation failed with status {status}"),
            )));
        }
        Ok(Self { api, handle })
    }

    pub fn version(&self) -> Option<String> {
        let value = unsafe { (self.api.version)() };
        (!value.is_null()).then(|| unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() })
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
        let mut options = unsafe { (self.api.options_default)() };
        options.size = std::mem::size_of::<RecognitionOptions>();
        options.request_id = request_id.as_ptr();
        options.language_code = language_code(request.language);
        options.enable_automatic_punctuation = true;
        let mut result = ptr::null_mut();
        let status = unsafe {
            (self.api.recognize_f32)(
                self.handle,
                &options,
                request.audio_samples.as_ptr(),
                request.audio_samples.len(),
                request.sample_rate as i32,
                &mut result,
            )
        };
        if status != STATUS_OK || result.is_null() {
            let detail = last_error(&self.api, String::new());
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
        let transcript = unsafe { (self.api.result_transcript)(result, 0) };
        let text = if transcript.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(transcript).to_string_lossy().into_owned() }
        };
        unsafe {
            (self.api.result_destroy)(result);
        }
        if text.trim().is_empty() {
            return Err(BackendError::EmptyRecording);
        }
        Ok(text)
    }
}

fn load_api(path: &Path) -> Result<NativeApi, BackendError> {
    let library = load_library(path).map_err(|error| {
        BackendError::MissingRuntimeModel(format!("could not load {}: {error:?}", path.display()))
    })?;
    unsafe {
        Ok(NativeApi {
            create: *symbol(&library, b"nemo_speech_asr_create\0")?,
            destroy: *symbol(&library, b"nemo_speech_asr_destroy\0")?,
            options_default: *symbol(&library, b"nemo_speech_asr_recognition_options_default\0")?,
            recognize_f32: *symbol(&library, b"nemo_speech_asr_recognize_f32\0")?,
            result_transcript: *symbol(&library, b"nemo_speech_asr_result_transcript\0")?,
            result_destroy: *symbol(&library, b"nemo_speech_asr_result_destroy\0")?,
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
    use super::find_library;
    use std::fs;

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
}

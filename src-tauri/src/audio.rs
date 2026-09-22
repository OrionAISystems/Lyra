use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use tracing::{debug, warn};

use crate::domain::{BackendError, CaptureEndpoint};

pub const MODEL_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct CapturedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_ms: u64,
    pub limit_reached: bool,
    pub endpoint: CaptureEndpoint,
    pub peak_level: f32,
    pub clipped_samples: u64,
    pub noise_floor: f32,
}

#[derive(Debug, Clone)]
pub struct CaptureOptions {
    pub device_id: Option<String>,
    pub max_duration_seconds: u32,
    pub silence_after_ms: Option<u64>,
    pub adaptive_audio: bool,
}

impl CaptureOptions {
    pub fn normalized(&self) -> Self {
        Self {
            device_id: self.device_id.clone(),
            max_duration_seconds: self.max_duration_seconds.clamp(1, 600),
            silence_after_ms: self
                .silence_after_ms
                .filter(|value| matches!(value, 1_000 | 1_500 | 2_000 | 3_000)),
            adaptive_audio: self.adaptive_audio,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaptureControl {
    stop: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    max_duration_ms: Arc<AtomicU64>,
}

impl CaptureControl {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn update_max_duration(&self, max_duration_seconds: u32) {
        self.max_duration_ms.store(
            u64::from(max_duration_seconds.clamp(1, 600)) * 1_000,
            Ordering::SeqCst,
        );
    }
}

pub struct ActiveCapture {
    control: CaptureControl,
    result: Receiver<Result<CapturedAudio, BackendError>>,
}

impl ActiveCapture {
    pub fn control(&self) -> CaptureControl {
        self.control.clone()
    }

    pub fn wait(self) -> Result<CapturedAudio, BackendError> {
        self.result.recv().unwrap_or_else(|_| {
            Err(BackendError::StreamFailure(
                "capture worker stopped unexpectedly".to_owned(),
            ))
        })
    }
}

pub trait AudioCapture: Send + Sync {
    fn list_devices(&self) -> Result<Vec<AudioDeviceInfo>, BackendError>;
    fn start(
        &self,
        options: CaptureOptions,
        on_level: Arc<dyn Fn(f32) + Send + Sync>,
    ) -> Result<ActiveCapture, BackendError>;
}

#[derive(Debug, Default)]
pub struct CpalAudioCapture;

impl CpalAudioCapture {
    pub fn new() -> Self {
        Self
    }

    fn find_device(
        &self,
        requested_id: Option<&str>,
    ) -> Result<(cpal::Device, bool), BackendError> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device
            .as_ref()
            .and_then(|device| device.name().ok());

        if requested_id.is_none() {
            return default_device.map(|device| (device, true)).ok_or_else(|| {
                BackendError::DeviceFailure("Windows has no default input device".to_owned())
            });
        }

        let requested_id = requested_id.unwrap_or_default();
        let devices = host
            .input_devices()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        for (index, device) in devices.enumerate() {
            let name = device
                .name()
                .unwrap_or_else(|_| "Unnamed microphone".to_owned());
            let id = format_device_id(&name, index);
            if requested_id == id || requested_id == name {
                let is_default = default_name.as_deref() == Some(name.as_str());
                return Ok((device, is_default));
            }
        }

        Err(BackendError::DeviceFailure(format!(
            "selected input device is unavailable: {requested_id}"
        )))
    }
}

impl AudioCapture for CpalAudioCapture {
    fn list_devices(&self) -> Result<Vec<AudioDeviceInfo>, BackendError> {
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let devices = host
            .input_devices()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;

        let mut result = Vec::new();
        for (index, device) in devices.enumerate() {
            let name = device
                .name()
                .unwrap_or_else(|_| "Unnamed microphone".to_owned());
            let config = match device.default_input_config() {
                Ok(config) => config,
                Err(error) => {
                    warn!(device = %name, error = %error, "input device has no default config");
                    continue;
                }
            };
            result.push(AudioDeviceInfo {
                id: format_device_id(&name, index),
                name: name.clone(),
                is_default: default_name.as_deref() == Some(name.as_str()),
                sample_rate: config.sample_rate().0,
                channels: config.channels(),
                format: format!("{:?}", config.sample_format()),
            });
        }
        Ok(result)
    }

    fn start(
        &self,
        options: CaptureOptions,
        on_level: Arc<dyn Fn(f32) + Send + Sync>,
    ) -> Result<ActiveCapture, BackendError> {
        let options = options.normalized();
        let (device, _) = self.find_device(options.device_id.as_deref())?;
        let supported_config = device
            .default_input_config()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.config();
        let channels = usize::from(config.channels.max(1));
        let native_sample_rate = config.sample_rate.0;
        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let stream_error = Arc::new(Mutex::new(None::<String>));
        let stop = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let max_duration_ms = Arc::new(AtomicU64::new(
            u64::from(options.max_duration_seconds) * 1_000,
        ));
        let control = CaptureControl {
            stop: Arc::clone(&stop),
            cancel: Arc::clone(&cancel),
            max_duration_ms: Arc::clone(&max_duration_ms),
        };
        let (result_tx, result_rx) = mpsc::channel();
        let samples_for_callback = Arc::clone(&samples);
        let error_for_callback = Arc::clone(&stream_error);
        let level_for_callback = Arc::clone(&on_level);
        let activity_level = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
        let last_voiced_ms = Arc::new(AtomicU64::new(0));
        let capture_clock = Instant::now();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let activity = Arc::clone(&activity_level);
                let voiced = Arc::clone(&last_voiced_ms);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        append_f32(
                            data,
                            channels,
                            &samples_for_callback,
                            &level_for_callback,
                            &activity,
                            &voiced,
                            capture_clock,
                        )
                    },
                    move |error| set_stream_error(&error_for_callback, error.to_string()),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let activity = Arc::clone(&activity_level);
                let voiced = Arc::clone(&last_voiced_ms);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        append_i16(
                            data,
                            channels,
                            &samples_for_callback,
                            &level_for_callback,
                            &activity,
                            &voiced,
                            capture_clock,
                        )
                    },
                    move |error| set_stream_error(&error_for_callback, error.to_string()),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let activity = Arc::clone(&activity_level);
                let voiced = Arc::clone(&last_voiced_ms);
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        append_u16(
                            data,
                            channels,
                            &samples_for_callback,
                            &level_for_callback,
                            &activity,
                            &voiced,
                            capture_clock,
                        )
                    },
                    move |error| set_stream_error(&error_for_callback, error.to_string()),
                    None,
                )
            }
            other => {
                return Err(BackendError::Unsupported(format!(
                    "microphone format {other:?}"
                )))
            }
        }
        .map_err(|error| BackendError::StreamFailure(error.to_string()))?;

        stream
            .play()
            .map_err(|error| BackendError::StreamFailure(error.to_string()))?;
        let silence_after_ms = options.silence_after_ms;
        let activity_level_for_worker = Arc::clone(&activity_level);
        let last_voiced_for_worker = Arc::clone(&last_voiced_ms);

        thread::Builder::new()
            .name("lyra-audio-capture".to_owned())
            .spawn(move || {
                let started = Instant::now();
                let mut limit_reached = false;
                let mut endpoint = CaptureEndpoint::Manual;
                let mut silence_detector = SilenceDetector::default();
                loop {
                    if cancel.load(Ordering::SeqCst) || stop.load(Ordering::SeqCst) {
                        break;
                    }
                    if started.elapsed().as_millis() as u64
                        >= max_duration_ms.load(Ordering::SeqCst)
                    {
                        limit_reached = true;
                        endpoint = CaptureEndpoint::MaximumDuration;
                        break;
                    }
                    if let Some(silence_after_ms) = silence_after_ms {
                        let level =
                            f32::from_bits(activity_level_for_worker.load(Ordering::Acquire));
                        let now_ms = started.elapsed().as_millis() as u64;
                        let last_voiced = last_voiced_for_worker.load(Ordering::Acquire);
                        if silence_detector.update(level, now_ms, last_voiced, silence_after_ms) {
                            endpoint = CaptureEndpoint::Silence;
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                drop(stream);

                if cancel.load(Ordering::SeqCst) {
                    let _ = result_tx.send(Err(BackendError::Cancellation));
                    return;
                }
                if let Some(error) = stream_error.lock().ok().and_then(|value| value.clone()) {
                    let _ = result_tx.send(Err(BackendError::StreamFailure(error)));
                    return;
                }

                let captured = samples
                    .lock()
                    .map(|value| value.clone())
                    .unwrap_or_default();
                if captured.is_empty() {
                    let _ = result_tx.send(Err(BackendError::EmptyRecording));
                    return;
                }
                let samples = resample_linear(&captured, native_sample_rate, MODEL_SAMPLE_RATE);
                let analysis = analyze_audio(&samples, MODEL_SAMPLE_RATE, options.adaptive_audio);
                let samples = analysis.samples;
                if samples.len() < (MODEL_SAMPLE_RATE as usize * MIN_VOICED_MS as usize / 1_000) {
                    let _ = result_tx.send(Err(BackendError::EmptyRecording));
                    return;
                }
                let duration_ms = ((samples.len() as u64) * 1_000) / u64::from(MODEL_SAMPLE_RATE);
                debug!(duration_ms, limit_reached, "audio capture finished");
                let _ = result_tx.send(Ok(CapturedAudio {
                    samples,
                    sample_rate: MODEL_SAMPLE_RATE,
                    duration_ms,
                    limit_reached,
                    endpoint,
                    peak_level: analysis.peak_level,
                    clipped_samples: analysis.clipped_samples,
                    noise_floor: analysis.noise_floor,
                }));
            })
            .map_err(|error| BackendError::StreamFailure(error.to_string()))?;

        Ok(ActiveCapture {
            control,
            result: result_rx,
        })
    }
}

fn format_device_id(name: &str, index: usize) -> String {
    format!("{name}::{index}")
}

fn set_stream_error(target: &Mutex<Option<String>>, error: String) {
    if let Ok(mut target) = target.lock() {
        *target = Some(error);
    }
}

fn append_f32(
    data: &[f32],
    channels: usize,
    samples: &Mutex<Vec<f32>>,
    on_level: &Arc<dyn Fn(f32) + Send + Sync>,
    activity_level: &AtomicU32,
    last_voiced_ms: &AtomicU64,
    capture_clock: Instant,
) {
    append_converted(
        data,
        channels,
        samples,
        on_level,
        activity_level,
        last_voiced_ms,
        capture_clock,
        |value| value,
    );
}

fn append_i16(
    data: &[i16],
    channels: usize,
    samples: &Mutex<Vec<f32>>,
    on_level: &Arc<dyn Fn(f32) + Send + Sync>,
    activity_level: &AtomicU32,
    last_voiced_ms: &AtomicU64,
    capture_clock: Instant,
) {
    append_converted(
        data,
        channels,
        samples,
        on_level,
        activity_level,
        last_voiced_ms,
        capture_clock,
        |value| f32::from(value) / 32_768.0,
    );
}

fn append_u16(
    data: &[u16],
    channels: usize,
    samples: &Mutex<Vec<f32>>,
    on_level: &Arc<dyn Fn(f32) + Send + Sync>,
    activity_level: &AtomicU32,
    last_voiced_ms: &AtomicU64,
    capture_clock: Instant,
) {
    append_converted(
        data,
        channels,
        samples,
        on_level,
        activity_level,
        last_voiced_ms,
        capture_clock,
        |value| (f32::from(value) - 32_768.0) / 32_768.0,
    );
}

fn append_converted<T: Copy>(
    data: &[T],
    channels: usize,
    samples: &Mutex<Vec<f32>>,
    on_level: &Arc<dyn Fn(f32) + Send + Sync>,
    activity_level: &AtomicU32,
    last_voiced_ms: &AtomicU64,
    capture_clock: Instant,
    convert: impl Fn(T) -> f32,
) {
    if let Ok(mut target) = samples.lock() {
        let mut sum = 0.0_f32;
        let mut frame_count = 0_u32;
        for frame in data.chunks(channels) {
            if frame.is_empty() {
                continue;
            }
            let mono = frame.iter().copied().map(&convert).sum::<f32>() / frame.len() as f32;
            sum += mono * mono;
            frame_count += 1;
            target.push(mono.clamp(-1.0, 1.0));
        }
        if frame_count > 0 {
            let level = (sum / frame_count as f32).sqrt().min(1.0);
            activity_level.store(level.to_bits(), Ordering::Release);
            if level >= SPEECH_RMS_THRESHOLD {
                last_voiced_ms.store(
                    capture_clock.elapsed().as_millis() as u64 + 1,
                    Ordering::Release,
                );
            }
            on_level(level);
        }
    }
}

pub const SPEECH_RMS_THRESHOLD: f32 = 0.025;
pub const MIN_VOICED_MS: u64 = 120;

#[derive(Debug, Default)]
pub struct SilenceDetector {
    voiced_since_ms: Option<u64>,
    speech_armed: bool,
    noise_floor: f32,
}

impl SilenceDetector {
    pub fn update(
        &mut self,
        level: f32,
        now_ms: u64,
        last_voiced_ms: u64,
        silence_after_ms: u64,
    ) -> bool {
        if !self.speech_armed && level < 0.08 {
            self.noise_floor = if self.noise_floor == 0.0 {
                level
            } else {
                self.noise_floor * 0.92 + level * 0.08
            };
        }
        let threshold = (self.noise_floor * 3.0)
            .clamp(0.012, 0.06)
            .max(SPEECH_RMS_THRESHOLD * 0.65);
        if level >= threshold {
            self.voiced_since_ms.get_or_insert(now_ms);
            if now_ms.saturating_sub(self.voiced_since_ms.unwrap_or(now_ms)) >= MIN_VOICED_MS {
                self.speech_armed = true;
            }
            return false;
        }
        self.voiced_since_ms = None;
        self.speech_armed
            && last_voiced_ms > 0
            && now_ms.saturating_add(1).saturating_sub(last_voiced_ms) >= silence_after_ms
    }
}

struct AudioAnalysis {
    samples: Vec<f32>,
    peak_level: f32,
    clipped_samples: u64,
    noise_floor: f32,
}

fn analyze_audio(samples: &[f32], sample_rate: u32, adaptive: bool) -> AudioAnalysis {
    if samples.is_empty() {
        return AudioAnalysis {
            samples: Vec::new(),
            peak_level: 0.0,
            clipped_samples: 0,
            noise_floor: 0.0,
        };
    }
    let calibration_len = samples.len().min((sample_rate / 4) as usize).max(1);
    let noise_floor = (samples[..calibration_len]
        .iter()
        .map(|sample| sample * sample)
        .sum::<f32>()
        / calibration_len as f32)
        .sqrt();
    let peak_level = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let clipped_samples = samples
        .iter()
        .filter(|sample| sample.abs() >= 0.985)
        .count() as u64;
    if !adaptive {
        return AudioAnalysis {
            samples: samples.to_vec(),
            peak_level,
            clipped_samples,
            noise_floor,
        };
    }
    let threshold = (noise_floor * 2.5).clamp(0.006, 0.045);
    let Some(first) = samples.iter().position(|sample| sample.abs() >= threshold) else {
        return AudioAnalysis {
            samples: Vec::new(),
            peak_level,
            clipped_samples,
            noise_floor,
        };
    };
    let last = samples
        .iter()
        .rposition(|sample| sample.abs() >= threshold)
        .unwrap_or(first);
    let padding = (sample_rate / 10) as usize;
    let start = first.saturating_sub(padding);
    let end = (last + padding + 1).min(samples.len());
    AudioAnalysis {
        samples: samples[start..end].to_vec(),
        peak_level,
        clipped_samples,
        noise_floor,
    }
}

pub fn resample_linear(input: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if input.is_empty() || source_rate == 0 || source_rate == target_rate {
        return input.to_vec();
    }
    let output_len =
        ((input.len() as u64) * u64::from(target_rate) / u64::from(source_rate)) as usize;
    let mut output = Vec::with_capacity(output_len.max(1));
    let ratio = source_rate as f64 / target_rate as f64;
    for index in 0..output_len {
        let source_position = index as f64 * ratio;
        let left = source_position.floor() as usize;
        let right = (left + 1).min(input.len() - 1);
        let fraction = (source_position - left as f64) as f32;
        output.push(input[left] * (1.0 - fraction) + input[right] * fraction);
    }
    output
}

pub fn encode_pcm_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * 32_767.0) as i16;
        wav.extend_from_slice(&value.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_changes_length_and_keeps_endpoints() {
        let input = vec![0.0, 1.0, 0.0, -1.0];
        let output = resample_linear(&input, 4, 8);
        assert_eq!(output.len(), 8);
        assert_eq!(output.first().copied(), Some(0.0));
        assert!((output[2] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn wav_has_pcm_header_and_payload() {
        let wav = encode_pcm_wav(&[0.0, 1.0], 16_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 48);
    }

    #[test]
    fn silence_detector_does_not_endpoint_before_voiced_interval() {
        let mut detector = SilenceDetector::default();
        assert!(!detector.update(0.0, 1_000, 0, 1_000));
        assert!(!detector.update(0.1, 1_010, 11, 1_000));
        assert!(!detector.update(0.1, 1_100, 101, 1_000));
        assert!(!detector.update(0.1, 1_130, 131, 1_000));
        assert!(!detector.update(0.0, 1_200, 1_131, 1_000));
        assert!(detector.update(0.0, 2_200, 1_131, 1_000));
    }

    #[test]
    fn capture_options_accept_only_supported_silence_values() {
        let options = CaptureOptions {
            device_id: None,
            max_duration_seconds: 999,
            silence_after_ms: Some(900),
            adaptive_audio: true,
        }
        .normalized();
        assert_eq!(options.max_duration_seconds, 600);
        assert_eq!(options.silence_after_ms, None);
    }

    #[test]
    fn capture_control_updates_dynamic_duration_atomically() {
        let control = CaptureControl {
            stop: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
            max_duration_ms: Arc::new(AtomicU64::new(120_000)),
        };
        control.update_max_duration(300);
        assert_eq!(control.max_duration_ms.load(Ordering::Acquire), 300_000);
        control.update_max_duration(60);
        assert_eq!(control.max_duration_ms.load(Ordering::Acquire), 60_000);
    }
}

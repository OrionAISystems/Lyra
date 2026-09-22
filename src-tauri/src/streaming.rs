//! Small, backend-agnostic pieces used by native streaming recognition.
//!
//! The audio callback must never call into the native recognizer.  It only
//! copies a bounded f32 chunk into [`F32ChunkProducer`]; a worker owns the
//! corresponding [`F32ChunkConsumer`] and drives the recognizer from there.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
};

use serde::Serialize;

use crate::domain::LanguagePreference;

/// A partial hypothesis produced before the native stream reaches its final
/// end-of-utterance result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialTranscript {
    pub text: String,
    pub audio_processed_ms: Option<u64>,
}

/// The single final hypothesis for a Lyra gesture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalTranscript {
    pub text: String,
    pub audio_processed_ms: Option<u64>,
}

/// A result pulled from a native streaming session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamingResult {
    Partial(PartialTranscript),
    Final(FinalTranscript),
}

impl StreamingResult {
    pub fn text(&self) -> &str {
        match self {
            Self::Partial(result) => &result.text,
            Self::Final(result) => &result.text,
        }
    }

    pub fn is_final(&self) -> bool {
        matches!(self, Self::Final(_))
    }

    pub fn audio_processed_ms(&self) -> Option<u64> {
        match self {
            Self::Partial(result) => result.audio_processed_ms,
            Self::Final(result) => result.audio_processed_ms,
        }
    }
}

/// Recognition options retained by a streaming session.
///
/// Context phrases are copied into the session and remain owned until the
/// native stream is closed.  This is important because the C ABI receives
/// borrowed pointers to the phrase array and strings.
#[derive(Debug, Clone, Default)]
pub struct StreamingRequest {
    pub request_id: String,
    pub language: LanguagePreference,
    pub context_phrases: Vec<String>,
    pub interim_results: bool,
}

impl StreamingRequest {
    pub fn new(request_id: impl Into<String>, language: LanguagePreference) -> Self {
        Self {
            request_id: request_id.into(),
            language,
            context_phrases: Vec::new(),
            interim_results: true,
        }
    }

    pub fn with_context_phrases<I, S>(mut self, phrases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.context_phrases = phrases
            .into_iter()
            .map(Into::into)
            .filter(|phrase| !phrase.trim().is_empty())
            .collect();
        self
    }

    pub fn final_only(mut self) -> Self {
        self.interim_results = false;
        self
    }
}

/// Keeps only the first final result for a gesture.  Lyra intentionally uses
/// one gesture/one final semantics; callers may still observe partials while
/// recording, but subsequent native finals are ignored.
#[derive(Debug, Default)]
pub struct FinalOnlyCollector {
    final_result: Option<FinalTranscript>,
}

impl FinalOnlyCollector {
    pub fn push(&mut self, result: StreamingResult) -> Option<FinalTranscript> {
        let StreamingResult::Final(result) = result else {
            return None;
        };
        if self.final_result.is_some() {
            return None;
        }
        self.final_result = Some(result.clone());
        Some(result)
    }

    pub fn result(&self) -> Option<&FinalTranscript> {
        self.final_result.as_ref()
    }

    pub fn into_result(self) -> Option<FinalTranscript> {
        self.final_result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    Enqueued,
    /// The bounded queue was full.  The producer never waits in this case.
    Dropped,
    /// The queue was cancelled or closed.
    Closed,
    /// Empty chunks are ignored without changing queue state.
    Empty,
    /// Rejecting non-finite values prevents invalid samples from crossing the
    /// native ABI boundary.
    RejectedNonFinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueClosed;

struct QueueInner {
    chunks: Mutex<VecDeque<Vec<f32>>>,
    wake: Condvar,
    capacity: usize,
    closed: AtomicBool,
    cancelled: AtomicBool,
    dropped: AtomicU64,
}

/// The callback-side handle.  `push` is bounded and never waits for the
/// consumer; full queues drop a chunk and expose the drop to the caller.
#[derive(Clone)]
pub struct F32ChunkProducer {
    inner: Arc<QueueInner>,
}

/// The worker-side handle.  Native calls belong on the consumer side, never
/// in the CPAL callback.
pub struct F32ChunkConsumer {
    inner: Arc<QueueInner>,
}

/// Construct a bounded f32 chunk queue.  A zero capacity is normalized to one
/// so a caller cannot accidentally create a queue that can never make
/// progress.
pub fn bounded_f32_chunks(capacity: usize) -> (F32ChunkProducer, F32ChunkConsumer) {
    let inner = Arc::new(QueueInner {
        chunks: Mutex::new(VecDeque::with_capacity(capacity.max(1))),
        wake: Condvar::new(),
        capacity: capacity.max(1),
        closed: AtomicBool::new(false),
        cancelled: AtomicBool::new(false),
        dropped: AtomicU64::new(0),
    });
    (
        F32ChunkProducer {
            inner: Arc::clone(&inner),
        },
        F32ChunkConsumer { inner },
    )
}

impl F32ChunkProducer {
    pub fn push(&self, samples: &[f32]) -> PushOutcome {
        if samples.is_empty() {
            return PushOutcome::Empty;
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return PushOutcome::RejectedNonFinite;
        }
        if self.inner.closed.load(Ordering::Acquire) || self.inner.cancelled.load(Ordering::Acquire)
        {
            return PushOutcome::Closed;
        }

        let mut chunks = match self.inner.chunks.try_lock() {
            Ok(chunks) => chunks,
            Err(std::sync::TryLockError::WouldBlock) => {
                self.inner.dropped.fetch_add(1, Ordering::AcqRel);
                return PushOutcome::Dropped;
            }
            Err(std::sync::TryLockError::Poisoned(_)) => {
                self.inner.cancelled.store(true, Ordering::Release);
                self.inner.wake.notify_all();
                return PushOutcome::Closed;
            }
        };
        if self.inner.closed.load(Ordering::Acquire) || self.inner.cancelled.load(Ordering::Acquire)
        {
            return PushOutcome::Closed;
        }
        if chunks.len() >= self.inner.capacity {
            self.inner.dropped.fetch_add(1, Ordering::AcqRel);
            return PushOutcome::Dropped;
        }
        chunks.push_back(samples.to_vec());
        drop(chunks);
        self.inner.wake.notify_one();
        PushOutcome::Enqueued
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
        self.inner.wake.notify_all();
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        if let Ok(mut chunks) = self.inner.chunks.lock() {
            chunks.clear();
        }
        self.inner.wake.notify_all();
    }

    pub fn dropped_chunks(&self) -> u64 {
        self.inner.dropped.load(Ordering::Acquire)
    }
}

impl F32ChunkConsumer {
    pub fn recv(&self) -> Result<Vec<f32>, QueueClosed> {
        let Ok(mut chunks) = self.inner.chunks.lock() else {
            return Err(QueueClosed);
        };
        loop {
            if let Some(chunk) = chunks.pop_front() {
                return Ok(chunk);
            }
            if self.inner.closed.load(Ordering::Acquire)
                || self.inner.cancelled.load(Ordering::Acquire)
            {
                return Err(QueueClosed);
            }
            chunks = match self.inner.wake.wait(chunks) {
                Ok(chunks) => chunks,
                Err(_) => return Err(QueueClosed),
            };
        }
    }

    pub fn try_recv(&self) -> Result<Option<Vec<f32>>, QueueClosed> {
        let Ok(mut chunks) = self.inner.chunks.lock() else {
            return Err(QueueClosed);
        };
        if let Some(chunk) = chunks.pop_front() {
            return Ok(Some(chunk));
        }
        if self.inner.closed.load(Ordering::Acquire) || self.inner.cancelled.load(Ordering::Acquire)
        {
            return Err(QueueClosed);
        }
        Ok(None)
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        if let Ok(mut chunks) = self.inner.chunks.lock() {
            chunks.clear();
        }
        self.inner.wake.notify_all();
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
        self.inner.wake.notify_all();
    }

    pub fn queued_chunks(&self) -> usize {
        self.inner
            .chunks
            .lock()
            .map(|chunks| chunks.len())
            .unwrap_or(0)
    }

    pub fn dropped_chunks(&self) -> u64 {
        self.inner.dropped.load(Ordering::Acquire)
    }
}

/// Alias emphasizing that this queue is intended for streaming audio.
pub type StreamingAudioProducer = F32ChunkProducer;
pub type StreamingAudioConsumer = F32ChunkConsumer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partials_are_not_published_by_final_only_collector() {
        let mut collector = FinalOnlyCollector::default();
        assert_eq!(
            collector.push(StreamingResult::Partial(PartialTranscript {
                text: "hel".to_owned(),
                audio_processed_ms: Some(120),
            })),
            None
        );
        let final_result = FinalTranscript {
            text: "hello".to_owned(),
            audio_processed_ms: Some(500),
        };
        assert_eq!(
            collector.push(StreamingResult::Final(final_result.clone())),
            Some(final_result.clone())
        );
        assert_eq!(collector.result(), Some(&final_result));
        assert_eq!(
            collector.push(StreamingResult::Final(FinalTranscript {
                text: "second".to_owned(),
                audio_processed_ms: None,
            })),
            None
        );
        assert_eq!(collector.into_result(), Some(final_result));
    }

    #[test]
    fn queue_is_bounded_and_reports_overflow_without_blocking() {
        let (producer, consumer) = bounded_f32_chunks(1);
        assert_eq!(producer.push(&[0.0, 0.5]), PushOutcome::Enqueued);
        assert_eq!(producer.push(&[0.25]), PushOutcome::Dropped);
        assert_eq!(producer.dropped_chunks(), 1);
        assert_eq!(consumer.queued_chunks(), 1);
        assert_eq!(consumer.try_recv().unwrap(), Some(vec![0.0, 0.5]));
        assert_eq!(consumer.try_recv().unwrap(), None);
    }

    #[test]
    fn queue_cancellation_wakes_consumer_and_rejects_future_chunks() {
        let (producer, consumer) = bounded_f32_chunks(2);
        assert_eq!(producer.push(&[0.25]), PushOutcome::Enqueued);
        producer.cancel();
        assert_eq!(consumer.try_recv(), Err(QueueClosed));

        let (producer, consumer) = bounded_f32_chunks(2);
        let waiter = std::thread::spawn(move || consumer.recv());
        producer.cancel();
        assert_eq!(waiter.join().unwrap(), Err(QueueClosed));
        assert_eq!(producer.push(&[1.0]), PushOutcome::Closed);
    }

    #[test]
    fn queue_rejects_non_finite_audio() {
        let (producer, consumer) = bounded_f32_chunks(1);
        assert_eq!(producer.push(&[f32::NAN]), PushOutcome::RejectedNonFinite);
        assert_eq!(consumer.try_recv().unwrap(), None);
    }
}

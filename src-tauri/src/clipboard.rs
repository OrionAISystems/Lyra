use std::thread;
use std::time::Duration;

use crate::domain::BackendError;
use crate::{
    personalization::{AppProfile, InjectionStrategy},
    windows::{focus_target, inject_ctrl_v, inject_shift_insert, TargetContext},
};

const MAX_CLIPBOARD_TEXT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ClipboardSnapshot {
    pub sequence: u32,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectionOutcome {
    pub restored_previous_text: bool,
    pub pasted: bool,
}

pub trait TextInjector: Send + Sync {
    fn insertion_context(&self, _target: TargetContext) -> Option<crate::domain::InsertionContext> {
        None
    }
    fn copy_text(&self, text: &str) -> Result<(), BackendError>;
    fn paste_into_target(
        &self,
        target: TargetContext,
        text: &str,
    ) -> Result<InjectionOutcome, BackendError>;
    fn paste_with_profile(
        &self,
        target: TargetContext,
        text: &str,
        profile: &AppProfile,
    ) -> Result<InjectionOutcome, BackendError> {
        let _ = profile;
        self.paste_into_target(target, text)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClipboardService;

impl ClipboardService {
    pub fn snapshot_text(&self) -> Result<ClipboardSnapshot, BackendError> {
        platform::snapshot_text()
    }

    pub fn set_text(&self, text: &str) -> Result<u32, BackendError> {
        platform::set_text(text)
    }

    fn set_text_if_sequence(
        &self,
        expected_sequence: u32,
        text: &str,
    ) -> Result<Option<u32>, BackendError> {
        platform::set_text_if_sequence(expected_sequence, text)
    }

    pub fn current_sequence(&self) -> u32 {
        platform::current_sequence()
    }

    pub fn restore_if_unchanged(&self, snapshot: &ClipboardSnapshot) -> Result<bool, BackendError> {
        match snapshot.text.as_deref() {
            Some(text) => self
                .set_text_if_sequence(snapshot.sequence, text)
                .map(|sequence| sequence.is_some()),
            None => Ok(false),
        }
    }

    pub fn copy_text(&self, text: &str) -> Result<(), BackendError> {
        self.set_text(text).map(|_| ())
    }

    pub fn paste_into_target(
        &self,
        target: TargetContext,
        text: &str,
    ) -> Result<InjectionOutcome, BackendError> {
        let snapshot = self.snapshot_text()?;
        let transcript_sequence = self.set_text(text)?;
        focus_target(target)?;
        if let Err(error) = inject_ctrl_v(target) {
            tracing::warn!(error = %error, "paste input failed; leaving transcript on clipboard");
            return Err(error);
        }
        thread::sleep(Duration::from_millis(40));
        let restored = self.restore_if_unchanged(&ClipboardSnapshot {
            sequence: transcript_sequence,
            text: snapshot.text.clone(),
        })?;
        Ok(InjectionOutcome {
            restored_previous_text: restored,
            pasted: true,
        })
    }

    pub fn paste_with_profile(
        &self,
        target: TargetContext,
        text: &str,
        profile: &AppProfile,
    ) -> Result<InjectionOutcome, BackendError> {
        if profile.injection_strategy == InjectionStrategy::ClipboardOnly {
            self.copy_text(text)?;
            return Ok(InjectionOutcome {
                restored_previous_text: false,
                pasted: false,
            });
        }
        let snapshot = self.snapshot_text()?;
        if let Err(error) = focus_target(target) {
            if let Err(clipboard_error) = self.set_text_if_sequence(snapshot.sequence, text) {
                tracing::warn!(
                    error = %clipboard_error,
                    "could not copy the full transcript after target focus was interrupted"
                );
            }
            return Err(error);
        }
        let chunks = if profile.chunk_long_text && text.chars().count() > profile.chunk_chars {
            chunk_text(text, profile.chunk_chars)
        } else {
            vec![text.to_owned()]
        };
        let mut last_sequence = 0;
        for chunk in chunks {
            last_sequence = self.set_text(&chunk)?;
            let injection = match profile.injection_strategy {
                InjectionStrategy::CtrlV => inject_ctrl_v(target),
                InjectionStrategy::ShiftInsert => inject_shift_insert(target),
                InjectionStrategy::ClipboardOnly => unreachable!(),
            };
            if let Err(error) = injection {
                if let Err(clipboard_error) = self.set_text_if_sequence(last_sequence, text) {
                    tracing::warn!(
                        error = %clipboard_error,
                        "could not restore the complete transcript to the clipboard after paste was interrupted"
                    );
                }
                return Err(error);
            }
            thread::sleep(Duration::from_millis(45));
        }
        let restored = if self.current_sequence() == last_sequence {
            self.restore_if_unchanged(&ClipboardSnapshot {
                sequence: last_sequence,
                text: snapshot.text,
            })?
        } else {
            false
        };
        Ok(InjectionOutcome {
            restored_previous_text: restored,
            pasted: true,
        })
    }
}

impl TextInjector for ClipboardService {
    fn insertion_context(&self, target: TargetContext) -> Option<crate::domain::InsertionContext> {
        crate::cursor_context::read(target)
    }
    fn copy_text(&self, text: &str) -> Result<(), BackendError> {
        ClipboardService::copy_text(self, text)
    }

    fn paste_into_target(
        &self,
        target: TargetContext,
        text: &str,
    ) -> Result<InjectionOutcome, BackendError> {
        ClipboardService::paste_into_target(self, target, text)
    }

    fn paste_with_profile(
        &self,
        target: TargetContext,
        text: &str,
        profile: &AppProfile,
    ) -> Result<InjectionOutcome, BackendError> {
        ClipboardService::paste_with_profile(self, target, text, profile)
    }
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let chars = text.chars().collect::<Vec<_>>();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn decode_unicode_text(units: &[u16]) -> Option<String> {
    let terminator = units.iter().position(|unit| *unit == 0)?;
    Some(String::from_utf16_lossy(&units[..terminator]))
}

fn bounded_unicode_unit_count(size_bytes: usize) -> Option<usize> {
    if size_bytes == 0 || size_bytes % 2 != 0 {
        return None;
    }
    Some((size_bytes / 2).min(MAX_CLIPBOARD_TEXT_BYTES / 2))
}

#[cfg(windows)]
mod platform {
    use std::ptr;

    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
                OpenClipboard, SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
            Ole::CF_UNICODETEXT,
        },
    };

    use super::{bounded_unicode_unit_count, decode_unicode_text, ClipboardSnapshot};
    use crate::domain::BackendError;

    pub fn current_sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn snapshot_text() -> Result<ClipboardSnapshot, BackendError> {
        let sequence = current_sequence();
        open_clipboard()?;
        let result = unsafe {
            let handle = GetClipboardData(CF_UNICODETEXT as u32);
            if handle.is_null() {
                Ok(None)
            } else {
                let pointer = GlobalLock(handle) as *const u16;
                if pointer.is_null() {
                    Err(BackendError::DeviceFailure(
                        "Windows could not read the current clipboard text".to_owned(),
                    ))
                } else {
                    let size_bytes = GlobalSize(handle);
                    let decoded =
                        if let Some(available_units) = bounded_unicode_unit_count(size_bytes) {
                            let units = std::slice::from_raw_parts(pointer, available_units);
                            decode_unicode_text(units).ok_or_else(|| {
                                BackendError::DeviceFailure(
                                    "Windows returned unterminated or oversized clipboard text"
                                        .to_owned(),
                                )
                            })
                        } else {
                            Err(BackendError::DeviceFailure(
                                "Windows returned malformed clipboard text".to_owned(),
                            ))
                        };
                    let _ = GlobalUnlock(handle);
                    decoded.map(Some)
                }
            }
        };
        unsafe {
            CloseClipboard();
        }
        result.map(|text| ClipboardSnapshot { sequence, text })
    }

    pub fn set_text(text: &str) -> Result<u32, BackendError> {
        open_clipboard()?;
        let result = write_text_while_open(text).map(|()| current_sequence());
        unsafe {
            CloseClipboard();
        }
        result
    }

    pub fn set_text_if_sequence(
        expected_sequence: u32,
        text: &str,
    ) -> Result<Option<u32>, BackendError> {
        open_clipboard()?;
        let result = if current_sequence() != expected_sequence {
            Ok(None)
        } else {
            write_text_while_open(text).map(|()| Some(current_sequence()))
        };
        unsafe {
            CloseClipboard();
        }
        result
    }

    fn write_text_while_open(text: &str) -> Result<(), BackendError> {
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0);
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
        if handle.is_null() {
            return Err(BackendError::DeviceFailure(
                "Windows could not allocate clipboard memory".to_owned(),
            ));
        }
        let pointer = unsafe { GlobalLock(handle) as *mut u16 };
        if pointer.is_null() {
            unsafe {
                GlobalFree(handle);
            }
            return Err(BackendError::DeviceFailure(
                "Windows could not lock clipboard memory".to_owned(),
            ));
        }
        unsafe {
            ptr::copy_nonoverlapping(utf16.as_ptr(), pointer, utf16.len());
            let _ = GlobalUnlock(handle);
            if EmptyClipboard() == 0 || SetClipboardData(CF_UNICODETEXT as u32, handle).is_null() {
                let _ = GlobalFree(handle);
                return Err(BackendError::DeviceFailure(
                    "Windows rejected the text clipboard update".to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn open_clipboard() -> Result<(), BackendError> {
        for _ in 0..5 {
            if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        Err(BackendError::DeviceFailure(
            "Windows clipboard is busy".to_owned(),
        ))
    }
}

#[cfg(not(windows))]
mod platform {
    use super::ClipboardSnapshot;
    use crate::domain::BackendError;

    pub fn current_sequence() -> u32 {
        0
    }
    pub fn snapshot_text() -> Result<ClipboardSnapshot, BackendError> {
        Err(BackendError::Unsupported(
            "Windows clipboard is unavailable".to_owned(),
        ))
    }
    pub fn set_text(_text: &str) -> Result<u32, BackendError> {
        Err(BackendError::Unsupported(
            "Windows clipboard is unavailable".to_owned(),
        ))
    }
    pub fn set_text_if_sequence(
        _expected_sequence: u32,
        _text: &str,
    ) -> Result<Option<u32>, BackendError> {
        Err(BackendError::Unsupported(
            "Windows clipboard is unavailable".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeClipboard {
        sequence: u32,
        text: Option<String>,
    }

    impl FakeClipboard {
        fn restore_if_unchanged(&mut self, snapshot: &ClipboardSnapshot) -> bool {
            if self.sequence != snapshot.sequence {
                return false;
            }
            self.text = snapshot.text.clone();
            self.sequence += 1;
            true
        }
    }

    #[test]
    fn sequence_guard_does_not_overwrite_other_application_changes() {
        let mut clipboard = FakeClipboard {
            sequence: 4,
            text: Some("before".to_owned()),
        };
        let snapshot = ClipboardSnapshot {
            sequence: clipboard.sequence,
            text: clipboard.text.clone(),
        };
        clipboard.sequence += 1;
        clipboard.text = Some("another app".to_owned());
        assert!(!clipboard.restore_if_unchanged(&snapshot));
        assert_eq!(clipboard.text.as_deref(), Some("another app"));
    }

    #[test]
    fn sequence_guard_restores_when_unchanged() {
        let mut clipboard = FakeClipboard {
            sequence: 4,
            text: Some("before".to_owned()),
        };
        let snapshot = ClipboardSnapshot {
            sequence: clipboard.sequence,
            text: clipboard.text.clone(),
        };
        clipboard.text = Some("transcript".to_owned());
        clipboard.sequence = snapshot.sequence;
        assert!(clipboard.restore_if_unchanged(&snapshot));
        assert_eq!(clipboard.text.as_deref(), Some("before"));
    }

    #[test]
    fn chunking_preserves_all_unicode_text() {
        assert_eq!(chunk_text("abΓδε", 2).concat(), "abΓδε");
    }

    #[test]
    fn bounded_unicode_text_decode_requires_a_terminator() {
        assert_eq!(decode_unicode_text(&[0]), Some(String::new()));
        assert_eq!(
            decode_unicode_text(&[65, 937, 0, 66]),
            Some("AΩ".to_owned())
        );
        assert_eq!(decode_unicode_text(&[65, 937]), None);
    }

    #[test]
    fn unicode_clipboard_size_must_be_nonzero_and_even() {
        assert_eq!(bounded_unicode_unit_count(0), None);
        assert_eq!(bounded_unicode_unit_count(3), None);
        assert_eq!(bounded_unicode_unit_count(4), Some(2));
        assert_eq!(
            bounded_unicode_unit_count(MAX_CLIPBOARD_TEXT_BYTES + 2),
            Some(MAX_CLIPBOARD_TEXT_BYTES / 2),
        );
    }
}

use std::thread;
use std::time::Duration;

use crate::domain::BackendError;
use crate::{
    personalization::{AppProfile, InjectionStrategy},
    windows::{focus_target, inject_ctrl_v, inject_shift_insert, TargetContext},
};

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

    pub fn current_sequence(&self) -> u32 {
        platform::current_sequence()
    }

    pub fn restore_if_unchanged(&self, snapshot: &ClipboardSnapshot) -> Result<bool, BackendError> {
        if self.current_sequence() != snapshot.sequence {
            return Ok(false);
        }
        match snapshot.text.as_deref() {
            Some(text) => {
                self.set_text(text)?;
                Ok(true)
            }
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
        if let Err(error) = inject_ctrl_v() {
            tracing::warn!(error = %error, "paste input failed; leaving transcript on clipboard");
            return Err(error);
        }
        thread::sleep(Duration::from_millis(40));
        let restored = if self.current_sequence() == transcript_sequence {
            self.restore_if_unchanged(&ClipboardSnapshot {
                sequence: transcript_sequence,
                text: snapshot.text.clone(),
            })?
        } else {
            false
        };
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
        focus_target(target)?;
        let chunks = if profile.chunk_long_text && text.chars().count() > profile.chunk_chars {
            chunk_text(text, profile.chunk_chars)
        } else {
            vec![text.to_owned()]
        };
        let mut last_sequence = 0;
        for chunk in chunks {
            last_sequence = self.set_text(&chunk)?;
            match profile.injection_strategy {
                InjectionStrategy::CtrlV => inject_ctrl_v()?,
                InjectionStrategy::ShiftInsert => inject_shift_insert()?,
                InjectionStrategy::ClipboardOnly => unreachable!(),
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

#[cfg(windows)]
mod platform {
    use std::{ptr, slice};

    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
                OpenClipboard, SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::CF_UNICODETEXT,
        },
    };

    use super::ClipboardSnapshot;
    use crate::domain::BackendError;

    pub fn current_sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn snapshot_text() -> Result<ClipboardSnapshot, BackendError> {
        let sequence = current_sequence();
        open_clipboard()?;
        let text = unsafe {
            let handle = GetClipboardData(CF_UNICODETEXT as u32);
            if handle.is_null() {
                None
            } else {
                let pointer = GlobalLock(handle) as *const u16;
                if pointer.is_null() {
                    None
                } else {
                    let mut length = 0;
                    while *pointer.add(length) != 0 {
                        length += 1;
                    }
                    let value = String::from_utf16_lossy(slice::from_raw_parts(pointer, length));
                    let _ = GlobalUnlock(handle);
                    Some(value)
                }
            }
        };
        unsafe {
            CloseClipboard();
        }
        Ok(ClipboardSnapshot { sequence, text })
    }

    pub fn set_text(text: &str) -> Result<u32, BackendError> {
        open_clipboard()?;
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0);
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
        if handle.is_null() {
            unsafe {
                CloseClipboard();
            }
            return Err(BackendError::DeviceFailure(
                "Windows could not allocate clipboard memory".to_owned(),
            ));
        }
        let pointer = unsafe { GlobalLock(handle) as *mut u16 };
        if pointer.is_null() {
            unsafe {
                GlobalFree(handle);
                CloseClipboard();
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
                CloseClipboard();
                return Err(BackendError::DeviceFailure(
                    "Windows rejected the text clipboard update".to_owned(),
                ));
            }
            CloseClipboard();
        }
        Ok(current_sequence())
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
}

//! Read-only, bounded accessibility context at the actual insertion caret.
//! Text never leaves the COM worker; callers receive only casing/spacing policy.

use crate::{domain::InsertionContext, windows::TargetContext};

pub fn initialize() {
    #[cfg(windows)]
    platform::initialize();
}

#[cfg(not(windows))]
pub fn read(_target: TargetContext) -> Option<InsertionContext> {
    None
}

#[cfg(windows)]
pub fn read(target: TargetContext) -> Option<InsertionContext> {
    platform::read(target)
}

#[cfg(windows)]
mod platform {
    use std::{
        sync::{mpsc, OnceLock},
        thread,
        time::{Duration, Instant},
    };

    use windows::Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        UI::Accessibility::{
            CUIAutomation8, IUIAutomation2, IUIAutomationElement, IUIAutomationTextPattern,
            IUIAutomationTextRange, TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start,
            TextUnit_Character, UIA_TextPatternId,
        },
    };

    use super::{InsertionContext, TargetContext};
    use crate::windows::target_is_foreground;

    const CONTEXT_CHARACTERS: i32 = 128;
    const QUERY_BUDGET: Duration = Duration::from_millis(250);

    struct Query {
        target: TargetContext,
        deadline: Instant,
        reply: mpsc::SyncSender<Option<InsertionContext>>,
    }

    static WORKER: OnceLock<Option<mpsc::SyncSender<Query>>> = OnceLock::new();

    pub fn initialize() {
        WORKER.get_or_init(start_worker);
    }

    pub fn read(target: TargetContext) -> Option<InsertionContext> {
        if !target_is_foreground(target) {
            return None;
        }
        let worker = WORKER.get_or_init(start_worker).as_ref()?;
        let (reply, response) = mpsc::sync_channel(1);
        worker
            .try_send(Query {
                target,
                deadline: Instant::now() + QUERY_BUDGET,
                reply,
            })
            .ok()?;
        let context = response.recv_timeout(QUERY_BUDGET).ok().flatten();
        target_is_foreground(target).then_some(context).flatten()
    }

    fn start_worker() -> Option<mpsc::SyncSender<Query>> {
        // One worker and one queued request bound resource use even if an app's provider hangs.
        let (sender, receiver) = mpsc::sync_channel::<Query>(1);
        thread::Builder::new()
            .name("lyra-cursor-context".to_owned())
            .spawn(move || {
                // UI Automation must run away from the application's UI thread, in its own MTA.
                if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
                    return;
                }
                struct ComApartment;
                impl Drop for ComApartment {
                    fn drop(&mut self) {
                        unsafe { CoUninitialize() };
                    }
                }
                let _apartment = ComApartment;
                let automation: windows::core::Result<IUIAutomation2> =
                    unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) };
                let Ok(automation) = automation else {
                    return;
                };
                if unsafe { automation.SetConnectionTimeout(100) }.is_err()
                    || unsafe { automation.SetTransactionTimeout(150) }.is_err()
                {
                    return;
                }
                for query in receiver {
                    if Instant::now() >= query.deadline {
                        continue;
                    }
                    let context = unsafe { query_context(&automation, query.target) }.ok();
                    if Instant::now() < query.deadline {
                        let _ = query.reply.try_send(context);
                    }
                }
            })
            .ok()?;
        Some(sender)
    }

    unsafe fn query_context(
        automation: &IUIAutomation2,
        target: TargetContext,
    ) -> windows::core::Result<InsertionContext> {
        let unknown =
            || windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32));
        if !target_is_foreground(target) {
            return Err(unknown());
        }
        let focused = automation.GetFocusedElement()?;
        if focused.CurrentProcessId()? as u32 != target.process_id
            || !focused.CurrentHasKeyboardFocus()?.as_bool()
            || focused.CurrentIsPassword()?.as_bool()
        {
            return Err(unknown());
        }
        let pattern: IUIAutomationTextPattern = focused.GetCurrentPatternAs(UIA_TextPatternId)?;
        let document = field_document(automation, &focused, &pattern)?;
        let selection = pattern.GetSelection()?;
        // A replacement selection or multiple selections have ambiguous insertion semantics.
        if selection.Length()? != 1 {
            return Err(unknown());
        }
        let caret = selection.GetElement(0)?;
        if caret.CompareEndpoints(
            TextPatternRangeEndpoint_Start,
            &caret,
            TextPatternRangeEndpoint_End,
        )? != 0
        {
            return Err(unknown());
        }
        if caret.CompareEndpoints(
            TextPatternRangeEndpoint_Start,
            &document,
            TextPatternRangeEndpoint_Start,
        )? < 0
            || caret.CompareEndpoints(
                TextPatternRangeEndpoint_End,
                &document,
                TextPatternRangeEndpoint_End,
            )? > 0
        {
            return Err(unknown());
        }
        let prefix = caret.Clone()?;
        prefix.MoveEndpointByUnit(
            TextPatternRangeEndpoint_Start,
            TextUnit_Character,
            -CONTEXT_CHARACTERS,
        )?;
        if prefix.CompareEndpoints(
            TextPatternRangeEndpoint_Start,
            &document,
            TextPatternRangeEndpoint_Start,
        )? < 0
        {
            prefix.MoveEndpointByRange(
                TextPatternRangeEndpoint_Start,
                &document,
                TextPatternRangeEndpoint_Start,
            )?;
        }
        // Request one extra unit to reject providers that expand Character to a larger unit.
        let text = prefix.GetText(CONTEXT_CHARACTERS + 1)?;
        if text.len() > CONTEXT_CHARACTERS as usize {
            return Err(unknown());
        }
        let context = InsertionContext::from_prefix(&text.to_string());
        let current_focus = automation.GetFocusedElement()?;
        if !automation
            .CompareElements(&focused, &current_focus)?
            .as_bool()
            || !target_is_foreground(target)
        {
            return Err(unknown());
        }
        let current_selection = pattern.GetSelection()?;
        if current_selection.Length()? != 1 {
            return Err(unknown());
        }
        let current_caret = current_selection.GetElement(0)?;
        if caret.CompareEndpoints(
            TextPatternRangeEndpoint_Start,
            &current_caret,
            TextPatternRangeEndpoint_Start,
        )? != 0
            || current_caret.CompareEndpoints(
                TextPatternRangeEndpoint_Start,
                &current_caret,
                TextPatternRangeEndpoint_End,
            )? != 0
        {
            return Err(unknown());
        }
        Ok(context)
    }

    unsafe fn field_document(
        automation: &IUIAutomation2,
        focused: &IUIAutomationElement,
        pattern: &IUIAutomationTextPattern,
    ) -> windows::core::Result<IUIAutomationTextRange> {
        // Chromium can expose a placeholder/container-owned TextPattern on an
        // edit. Even RangeFromChild(edit) can return that same ambiguous range,
        // so its selection/lookback are not evidence of text in the field.
        let document = pattern.DocumentRange()?;
        if !automation
            .CompareElements(focused, &document.GetEnclosingElement()?)?
            .as_bool()
        {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80004005u32 as i32,
            )));
        }
        Ok(document)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "read-only live Codex provider check; requires explicit HWND and PID"]
        fn delegated_composer_range_is_rejected_without_reading_text() {
            use windows::Win32::{
                Foundation::HWND,
                System::Variant::VARIANT,
                UI::Accessibility::{TreeScope_Descendants, UIA_ClassNamePropertyId},
            };
            let hwnd: usize = std::env::var("LYRA_COMPOSER_PROBE_HWND")
                .expect("explicit composer HWND required")
                .parse()
                .unwrap();
            let pid: i32 = std::env::var("LYRA_COMPOSER_PROBE_PID")
                .expect("explicit composer PID required")
                .parse()
                .unwrap();
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
                let result = (|| -> windows::core::Result<()> {
                    let automation: IUIAutomation2 =
                        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
                    automation.SetConnectionTimeout(100)?;
                    automation.SetTransactionTimeout(150)?;
                    let root = automation.ElementFromHandle(HWND(hwnd as *mut _))?;
                    assert_eq!(root.CurrentProcessId()?, pid);
                    let condition = automation.CreatePropertyCondition(
                        UIA_ClassNamePropertyId,
                        &VARIANT::from("ProseMirror"),
                    )?;
                    let field = root.FindFirst(TreeScope_Descendants, &condition)?;
                    assert_eq!(field.CurrentProcessId()?, pid);
                    assert!(!field.CurrentIsPassword()?.as_bool());
                    let pattern: IUIAutomationTextPattern =
                        field.GetCurrentPatternAs(UIA_TextPatternId)?;
                    let document = pattern.DocumentRange()?;
                    assert!(!automation
                        .CompareElements(&field, &document.GetEnclosingElement()?)?
                        .as_bool());
                    assert!(pattern
                        .RangeFromChild(&field)?
                        .Compare(&document)?
                        .as_bool());
                    assert!(field_document(&automation, &field, &pattern).is_err());
                    // The same fallback used by production must restore a clean opening.
                    assert_eq!(
                        crate::domain::process_transcription_text_with_context(
                            " and this is dictated \u{200b}",
                            None,
                            &[],
                            crate::domain::TextProcessingOptions {
                                formatting_commands_enabled: true,
                                self_correction_enabled: true,
                                corrections: &[],
                                snippets: &[],
                            },
                        ),
                        "And this is dictated"
                    );
                    Ok(())
                })();
                CoUninitialize();
                result.unwrap();
            }
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "opens synthetic Windows fields; run scripts/cursor-context-fixture.ps1"]
    fn native_cursor_context_fixture() {
        // Only inspect the synthetic process explicitly supplied by the fixture runner.
        let target = TargetContext {
            hwnd: std::env::var("LYRA_CURSOR_FIXTURE_HWND")
                .expect("fixture HWND required")
                .parse()
                .unwrap(),
            process_id: std::env::var("LYRA_CURSOR_FIXTURE_PID")
                .expect("fixture PID required")
                .parse()
                .unwrap(),
        };
        let case = std::env::var("LYRA_CURSOR_FIXTURE_CASE").expect("fixture case required");
        assert!(
            crate::windows::target_is_foreground(target),
            "synthetic fixture must be foreground"
        );
        let clipboard_sequence = crate::clipboard::ClipboardService.current_sequence();
        initialize();
        let actual = read(target);
        let expected = match case.as_str() {
            "empty" => Some(InsertionContext::from_prefix("")),
            "continuation" | "middle" | "bounded" => {
                Some(InsertionContext::from_prefix("typed text "))
            }
            "boundary" => Some(InsertionContext::from_prefix("finished. ")),
            "selection" | "password" | "unsupported" => None,
            _ => panic!("unrecognized fixture case"),
        };
        assert_eq!(actual, expected, "{case}");
        assert_eq!(
            crate::clipboard::ClipboardService.current_sequence(),
            clipboard_sequence,
            "context reading must not modify the clipboard"
        );
        if let Some(context) = actual {
            let output = crate::domain::process_transcription_text_with_context(
                " and this is dictated \u{200b}",
                Some(context),
                &[],
                crate::domain::TextProcessingOptions {
                    formatting_commands_enabled: true,
                    self_correction_enabled: true,
                    corrections: &[],
                    snippets: &[],
                },
            );
            assert_eq!(
                output,
                if context.continues_sentence {
                    "and this is dictated"
                } else {
                    "And this is dictated"
                }
            );
        }
        assert!(
            read(TargetContext {
                process_id: target.process_id + 1,
                ..target
            })
            .is_none(),
            "mismatched process must not be inspected"
        );
    }
}

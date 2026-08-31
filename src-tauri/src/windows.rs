#[derive(Debug, Clone, Copy)]
pub struct TargetContext {
    pub hwnd: isize,
}

impl TargetContext {
    pub fn is_valid(self) -> bool {
        self.hwnd != 0
    }
}

#[cfg(windows)]
mod platform {
    use std::{mem::size_of, thread, time::Duration};

    use tauri::{PhysicalPosition, WebviewWindow};
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
            },
            WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow},
        },
    };

    use super::TargetContext;
    use crate::domain::BackendError;

    pub fn capture_foreground_target() -> Result<TargetContext, BackendError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return Err(BackendError::DeviceFailure(
                "Windows did not expose a foreground target".to_owned(),
            ));
        }
        Ok(TargetContext {
            hwnd: hwnd as isize,
        })
    }

    pub fn target_is_alive(target: TargetContext) -> bool {
        unsafe { IsWindow(target.hwnd as HWND) != 0 }
    }

    pub fn focus_target(target: TargetContext) -> Result<(), BackendError> {
        if !target_is_alive(target) {
            return Err(BackendError::DeviceFailure(
                "the original target window has closed".to_owned(),
            ));
        }
        let focused = unsafe { SetForegroundWindow(target.hwnd as HWND) };
        if focused == 0 {
            return Err(BackendError::DeviceFailure(
                "Windows refused to restore the original target focus".to_owned(),
            ));
        }
        thread::sleep(Duration::from_millis(35));
        Ok(())
    }

    pub fn inject_ctrl_v() -> Result<(), BackendError> {
        let mut inputs = [
            keyboard_input(VK_CONTROL as u16, 0),
            keyboard_input(VK_V as u16, 0),
            keyboard_input(VK_V as u16, KEYEVENTF_KEYUP),
            keyboard_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
        ];
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        if sent != inputs.len() as u32 {
            return Err(BackendError::DeviceFailure(
                "Windows did not accept the paste input; the transcript remains on the clipboard"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn keyboard_input(key: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: 1,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    pub fn place_bar_on_target_monitor(
        bar: &WebviewWindow,
        target: TargetContext,
    ) -> Result<(), BackendError> {
        let monitor = unsafe { MonitorFromWindow(target.hwnd as HWND, MONITOR_DEFAULTTONEAREST) };
        if monitor.is_null() {
            return Err(BackendError::DeviceFailure(
                "could not resolve the target monitor".to_owned(),
            ));
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return Err(BackendError::DeviceFailure(
                "could not read the target monitor work area".to_owned(),
            ));
        }
        let size = bar
            .inner_size()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        let width = i32::try_from(size.width).unwrap_or(190);
        let height = i32::try_from(size.height).unwrap_or(42);
        let work_width = info.rcWork.right - info.rcWork.left;
        let x = info.rcWork.left + (work_width - width) / 2;
        let y = info.rcWork.bottom - height - 16;
        bar.set_position(PhysicalPosition::new(x, y))
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use super::TargetContext;
    use crate::domain::BackendError;

    pub fn capture_foreground_target() -> Result<TargetContext, BackendError> {
        Err(BackendError::Unsupported(
            "Lyra v1 targets Windows x64".to_owned(),
        ))
    }

    pub fn target_is_alive(_target: TargetContext) -> bool {
        false
    }
    pub fn focus_target(_target: TargetContext) -> Result<(), BackendError> {
        Err(BackendError::Unsupported(
            "Windows focus restore is unavailable".to_owned(),
        ))
    }
    pub fn inject_ctrl_v() -> Result<(), BackendError> {
        Err(BackendError::Unsupported(
            "Windows input injection is unavailable".to_owned(),
        ))
    }
    pub fn place_bar_on_target_monitor(
        _bar: &tauri::WebviewWindow,
        _target: TargetContext,
    ) -> Result<(), BackendError> {
        Ok(())
    }
}

pub use platform::{
    capture_foreground_target, focus_target, inject_ctrl_v, place_bar_on_target_monitor,
};

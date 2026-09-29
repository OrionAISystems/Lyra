use serde::Serialize;

use crate::settings::BarPlacement;

#[derive(Debug, Clone, Copy)]
pub struct TargetContext {
    pub hwnd: isize,
    pub process_id: u32,
}

fn target_identity_matches(
    target: TargetContext,
    observed_hwnd: isize,
    observed_process_id: u32,
) -> bool {
    target.hwnd != 0
        && target.process_id != 0
        && observed_hwnd == target.hwnd
        && observed_process_id == target.process_id
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BarPlacementStatus {
    pub placement: BarPlacement,
    pub monitor_id: Option<String>,
    pub monitor_index: Option<u32>,
    pub fallback: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: String,
    pub name: Option<String>,
    pub index: u32,
    pub primary: bool,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    pub work_area_x: i32,
    pub work_area_y: i32,
    pub work_area_width: u32,
    pub work_area_height: u32,
    pub scale_factor: f64,
}

pub fn monitor_id(monitor: &tauri::Monitor, index: usize) -> String {
    monitor
        .name()
        .cloned()
        .unwrap_or_else(|| format!("monitor-{}", index + 1))
}

pub fn resolve_saved_monitor_index(
    monitor_ids: &[String],
    saved_id: Option<&str>,
    saved_index: Option<u32>,
) -> Option<usize> {
    saved_id
        .and_then(|id| monitor_ids.iter().position(|candidate| candidate == id))
        .or_else(|| saved_index.and_then(|index| index.checked_sub(1).map(|index| index as usize)))
        .filter(|index| *index < monitor_ids.len())
}

pub fn list_displays(
    bar: &tauri::WebviewWindow,
) -> Result<Vec<DisplayInfo>, crate::domain::BackendError> {
    let monitors = bar
        .available_monitors()
        .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    let primary_position = bar
        .primary_monitor()
        .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?
        .map(|monitor| *monitor.position());
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let work_area = monitor.work_area();
            DisplayInfo {
                id: monitor_id(monitor, index),
                name: monitor.name().cloned(),
                index: index as u32 + 1,
                primary: primary_position.is_some_and(|position| position == *monitor.position()),
                position_x: monitor.position().x,
                position_y: monitor.position().y,
                width: monitor.size().width,
                height: monitor.size().height,
                work_area_x: work_area.position.x,
                work_area_y: work_area.position.y,
                work_area_width: work_area.size.width,
                work_area_height: work_area.size.height,
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect())
}

pub fn place_bar(
    bar: &tauri::WebviewWindow,
    placement: BarPlacement,
    saved_monitor_id: Option<&str>,
    saved_monitor_index: Option<u32>,
    target: Option<TargetContext>,
) -> Result<BarPlacementStatus, crate::domain::BackendError> {
    platform::place_bar(
        bar,
        placement,
        saved_monitor_id,
        saved_monitor_index,
        target,
    )
}

#[cfg(windows)]
mod platform {
    use std::{mem::size_of, thread, time::Duration};

    use tauri::{PhysicalPosition, WebviewWindow};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_INSERT,
                VK_SHIFT, VK_V,
            },
            WindowsAndMessaging::{
                GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
            },
        },
    };

    use super::{target_identity_matches, BarPlacementStatus, TargetContext};
    use crate::{domain::BackendError, settings::BarPlacement};

    pub fn capture_foreground_target() -> Result<TargetContext, BackendError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return Err(BackendError::DeviceFailure(
                "Windows did not expose a foreground target".to_owned(),
            ));
        }
        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        let target = TargetContext {
            hwnd: hwnd as isize,
            process_id,
        };
        if !target_identity_matches(target, hwnd as isize, process_id) {
            return Err(BackendError::DeviceFailure(
                "Windows did not expose a valid foreground target process".to_owned(),
            ));
        }
        Ok(target)
    }

    pub fn target_is_alive(target: TargetContext) -> bool {
        let hwnd = target.hwnd as HWND;
        if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
            return false;
        }
        let mut process_id = 0;
        let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
        thread_id != 0 && target_identity_matches(target, hwnd as isize, process_id)
    }

    fn target_is_foreground(target: TargetContext) -> bool {
        if !target_is_alive(target) {
            return false;
        }
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_null() || foreground as isize != target.hwnd {
            return false;
        }
        let mut process_id = 0;
        let thread_id = unsafe { GetWindowThreadProcessId(foreground, &mut process_id) };
        thread_id != 0 && target_identity_matches(target, foreground as isize, process_id)
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
        if !target_is_foreground(target) {
            return Err(BackendError::DeviceFailure(
                "the original target is no longer in the foreground".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn inject_ctrl_v(target: TargetContext) -> Result<(), BackendError> {
        let mut inputs = [
            keyboard_input(VK_CONTROL as u16, 0),
            keyboard_input(VK_V as u16, 0),
            keyboard_input(VK_V as u16, KEYEVENTF_KEYUP),
            keyboard_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
        ];
        if !target_is_foreground(target) {
            return Err(BackendError::DeviceFailure(
                "the original target lost focus before paste input".to_owned(),
            ));
        }
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

    pub fn inject_shift_insert(target: TargetContext) -> Result<(), BackendError> {
        let mut inputs = [
            keyboard_input(VK_SHIFT as u16, 0),
            keyboard_input(VK_INSERT as u16, 0),
            keyboard_input(VK_INSERT as u16, KEYEVENTF_KEYUP),
            keyboard_input(VK_SHIFT as u16, KEYEVENTF_KEYUP),
        ];
        if !target_is_foreground(target) {
            return Err(BackendError::DeviceFailure(
                "the original target lost focus before paste input".to_owned(),
            ));
        }
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        if sent != inputs.len() as u32 {
            return Err(BackendError::DeviceFailure(
                "Windows did not accept Shift+Insert; the transcript remains on the clipboard"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub fn target_executable(target: TargetContext) -> Option<String> {
        if target.process_id == 0 {
            return None;
        }
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, target.process_id) };
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 32_768];
        let mut length = buffer.len() as u32;
        let success =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) }
                != 0;
        unsafe { CloseHandle(process) };
        if !success {
            return None;
        }
        std::path::Path::new(&String::from_utf16_lossy(&buffer[..length as usize]))
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase())
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

    pub fn place_bar(
        bar: &WebviewWindow,
        placement: BarPlacement,
        saved_monitor_id: Option<&str>,
        saved_monitor_index: Option<u32>,
        target: Option<TargetContext>,
    ) -> Result<BarPlacementStatus, BackendError> {
        let monitors = bar
            .available_monitors()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        let primary = bar
            .primary_monitor()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?
            .or_else(|| monitors.first().cloned())
            .ok_or_else(|| BackendError::DeviceFailure("no display is available".to_owned()))?;
        let mut fallback = false;
        let mut message = None;
        let selected = match placement {
            BarPlacement::PrimaryDisplay => primary.clone(),
            BarPlacement::FollowActiveWindow => target
                .and_then(active_monitor_position)
                .and_then(|position| {
                    monitors
                        .iter()
                        .find(|monitor| {
                            monitor.position().x == position.0 && monitor.position().y == position.1
                        })
                        .cloned()
                })
                .unwrap_or_else(|| primary.clone()),
            BarPlacement::FixedMonitor => saved_monitor_id
                .and_then(|id| {
                    monitors
                        .iter()
                        .enumerate()
                        .find(|(index, monitor)| id == super::monitor_id(monitor, *index))
                        .map(|(_, monitor)| monitor.clone())
                })
                .or_else(|| {
                    let monitor_ids = monitors
                        .iter()
                        .enumerate()
                        .map(|(index, monitor)| super::monitor_id(monitor, index))
                        .collect::<Vec<_>>();
                    super::resolve_saved_monitor_index(&monitor_ids, None, saved_monitor_index)
                        .and_then(|index| monitors.get(index).cloned())
                })
                .unwrap_or_else(|| {
                    fallback = true;
                    message =
                        Some("Saved monitor is unavailable; using the primary display".to_owned());
                    primary.clone()
                }),
        };
        let size = bar
            .inner_size()
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        let width = i32::try_from(size.width).unwrap_or(190);
        let height = i32::try_from(size.height).unwrap_or(42);
        let work_area = selected.work_area();
        let x = work_area.position.x
            + (i32::try_from(work_area.size.width).unwrap_or(width) - width) / 2;
        let y = work_area.position.y + i32::try_from(work_area.size.height).unwrap_or(height)
            - height
            - 16;
        bar.set_position(PhysicalPosition::new(x, y))
            .map_err(|error| BackendError::DeviceFailure(error.to_string()))?;
        Ok(BarPlacementStatus {
            placement,
            monitor_id: Some(super::monitor_id(
                &selected,
                monitors
                    .iter()
                    .position(|monitor| {
                        monitor.position() == selected.position()
                            && monitor.size() == selected.size()
                    })
                    .unwrap_or(0),
            )),
            monitor_index: monitors
                .iter()
                .position(|monitor| {
                    monitor.position() == selected.position() && monitor.size() == selected.size()
                })
                .map(|index| index as u32 + 1),
            fallback,
            message,
        })
    }

    fn active_monitor_position(target: TargetContext) -> Option<(i32, i32)> {
        let monitor = unsafe { MonitorFromWindow(target.hwnd as HWND, MONITOR_DEFAULTTONEAREST) };
        if monitor.is_null() {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        (unsafe { GetMonitorInfoW(monitor, &mut info) } != 0)
            .then_some((info.rcMonitor.left, info.rcMonitor.top))
    }
}

#[cfg(not(windows))]
mod platform {
    use super::TargetContext;
    use crate::{domain::BackendError, settings::BarPlacement};

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
    pub fn inject_ctrl_v(_target: TargetContext) -> Result<(), BackendError> {
        Err(BackendError::Unsupported(
            "Windows input injection is unavailable".to_owned(),
        ))
    }
    pub fn inject_shift_insert(target: TargetContext) -> Result<(), BackendError> {
        inject_ctrl_v(target)
    }
    pub fn target_executable(_target: TargetContext) -> Option<String> {
        None
    }
    pub fn place_bar(
        _bar: &tauri::WebviewWindow,
        placement: BarPlacement,
        _saved_monitor_id: Option<&str>,
        _saved_monitor_index: Option<u32>,
        _target: Option<TargetContext>,
    ) -> Result<super::BarPlacementStatus, BackendError> {
        Ok(super::BarPlacementStatus {
            placement,
            monitor_id: None,
            monitor_index: None,
            fallback: false,
            message: None,
        })
    }
}

pub use platform::{
    capture_foreground_target, focus_target, inject_ctrl_v, inject_shift_insert, target_executable,
    target_is_alive,
};

pub fn capture_external_foreground_target(excluded_process_id: u32) -> Option<TargetContext> {
    capture_foreground_target()
        .ok()
        .filter(|target| target.process_id != excluded_process_id)
}

/// Grow the transparent bar window upward so its context menu remains on screen.
/// The lower edge stays fixed, including when placement is recalculated mid-session.
pub fn set_bar_menu_open(
    bar: &tauri::WebviewWindow,
    open: bool,
) -> Result<(), crate::domain::BackendError> {
    use tauri::{LogicalSize, PhysicalPosition};

    let target_height = if open { 162.0 } else { 42.0 };
    let scale = bar
        .scale_factor()
        .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    let current_size = bar
        .inner_size()
        .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    let target_physical_height = (target_height * scale).round() as i32;
    let height_delta = current_size.height as i32 - target_physical_height;
    if height_delta == 0 {
        return Ok(());
    }
    let position = bar
        .outer_position()
        .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    let anchored_position = PhysicalPosition::new(position.x, position.y + height_delta);
    let size = LogicalSize::new(190.0, target_height);
    if open {
        bar.set_position(anchored_position)
            .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
        bar.set_size(size)
            .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    } else {
        bar.set_size(size)
            .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
        bar.set_position(anchored_position)
            .map_err(|error| crate::domain::BackendError::DeviceFailure(error.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{resolve_saved_monitor_index, target_identity_matches, TargetContext};

    #[test]
    fn monitor_identity_wins_and_ordinal_is_the_fallback() {
        let ids = vec!["DISPLAY2".to_owned(), "DISPLAY1".to_owned()];
        assert_eq!(
            resolve_saved_monitor_index(&ids, Some("DISPLAY1"), Some(1)),
            Some(1)
        );
        assert_eq!(
            resolve_saved_monitor_index(&ids, Some("missing"), Some(2)),
            Some(1)
        );
        assert_eq!(
            resolve_saved_monitor_index(&ids, Some("missing"), None),
            None
        );
    }

    #[test]
    fn target_guard_rejects_foreground_or_process_mismatch() {
        let target = TargetContext {
            hwnd: 0x1234,
            process_id: 77,
        };
        assert!(target_identity_matches(target, 0x1234, 77));
        assert!(!target_identity_matches(target, 0x5678, 77));
        assert!(!target_identity_matches(target, 0x1234, 88));
        assert!(!target_identity_matches(
            TargetContext {
                hwnd: 0x1234,
                process_id: 0,
            },
            0x1234,
            0,
        ));
    }
}

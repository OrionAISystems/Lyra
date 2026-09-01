use serde::Serialize;

use crate::settings::BarPlacement;

#[derive(Debug, Clone, Copy)]
pub struct TargetContext {
    pub hwnd: isize,
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

    use super::BarPlacementStatus;
    use super::TargetContext;
    use crate::{domain::BackendError, settings::BarPlacement};

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
    pub fn inject_ctrl_v() -> Result<(), BackendError> {
        Err(BackendError::Unsupported(
            "Windows input injection is unavailable".to_owned(),
        ))
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

pub use platform::{capture_foreground_target, focus_target, inject_ctrl_v};

#[cfg(test)]
mod tests {
    use super::resolve_saved_monitor_index;

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
}

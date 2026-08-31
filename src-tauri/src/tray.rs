use tauri::{menu::MenuItem, AppHandle, Manager, Wry};

use crate::updater::{UpdateState, UpdateStatus};

#[derive(Clone)]
pub struct TrayController {
    update_item: MenuItem<Wry>,
}

impl TrayController {
    pub fn new(update_item: MenuItem<Wry>) -> Self {
        Self { update_item }
    }

    pub fn set_update_status(&self, status: &UpdateStatus) {
        let label = match status.state {
            UpdateState::Available => status
                .available_version
                .as_deref()
                .map(|version| format!("Update available · v{version}"))
                .unwrap_or_else(|| "Check for updates".to_owned()),
            UpdateState::Checking => "Checking for updates…".to_owned(),
            UpdateState::Downloading => "Downloading Lyra update…".to_owned(),
            UpdateState::Installing => "Installing Lyra update…".to_owned(),
            UpdateState::Error => "Check for updates".to_owned(),
            UpdateState::Idle | UpdateState::UpToDate => "Check for updates".to_owned(),
        };
        let _ = self.update_item.set_text(label);
    }
}

pub fn update_tray_status(app: &AppHandle, status: &UpdateStatus) {
    if let Some(controller) = app.try_state::<TrayController>() {
        controller.set_update_status(status);
    }
}

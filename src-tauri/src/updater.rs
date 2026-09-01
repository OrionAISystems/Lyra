use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;
use tracing::{info, warn};

pub const UPDATE_STATUS_EVENT: &str = "lyra://update-status";

pub type UpdateFuture<T> = Pin<Box<dyn Future<Output = Result<T, UpdateError>> + Send>>;
pub type UpdateProgressCallback = Arc<dyn Fn(UpdateProgress) + Send + Sync>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Installing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub state: UpdateState,
    pub current_version: String,
    pub available_version: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub progress: Option<f32>,
    pub last_checked_at: Option<String>,
    pub error: Option<String>,
}

impl UpdateStatus {
    pub fn idle(current_version: impl Into<String>) -> Self {
        Self {
            state: UpdateState::Idle,
            current_version: current_version.into(),
            available_version: None,
            notes: None,
            published_at: None,
            progress: None,
            last_checked_at: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateProgress {
    pub phase: UpdateState,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub trait UpdateProvider: Send + Sync {
    fn check(&self) -> UpdateFuture<Option<UpdateManifest>>;

    fn install(&self, version: String, progress: UpdateProgressCallback) -> UpdateFuture<()>;
}

pub trait UpdateEventSink: Send + Sync {
    fn emit(&self, status: &UpdateStatus);
}

#[derive(Clone)]
pub struct UpdateManager {
    status: Arc<Mutex<UpdateStatus>>,
    operation_in_flight: Arc<AtomicBool>,
    deferred_version: Arc<Mutex<Option<String>>>,
    provider: Arc<dyn UpdateProvider>,
    sink: Arc<dyn UpdateEventSink>,
    installation_gate: Arc<AtomicBool>,
}

impl UpdateManager {
    pub fn new(app: &AppHandle) -> Self {
        Self::with_provider(
            Arc::new(TauriUpdateProvider::new(app.clone())),
            Arc::new(TauriUpdateEventSink::new(app.clone())),
            env!("CARGO_PKG_VERSION").to_owned(),
        )
    }

    pub fn with_provider(
        provider: Arc<dyn UpdateProvider>,
        sink: Arc<dyn UpdateEventSink>,
        current_version: impl Into<String>,
    ) -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::idle(current_version))),
            operation_in_flight: Arc::new(AtomicBool::new(false)),
            deferred_version: Arc::new(Mutex::new(None)),
            provider,
            sink,
            installation_gate: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| UpdateStatus::idle(env!("CARGO_PKG_VERSION")))
    }

    pub async fn check_for_updates(&self) -> UpdateStatus {
        if self
            .operation_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return self.status();
        }

        let mut checking = self.status();
        checking.state = UpdateState::Checking;
        checking.progress = None;
        checking.error = None;
        self.publish(checking);

        let result = self.provider.check().await;
        let mut next = self.status();
        next.last_checked_at = Some(now_timestamp());
        next.progress = None;
        next.error = None;
        match result {
            Ok(Some(manifest))
                if is_newer_stable_version(&next.current_version, &manifest.version) =>
            {
                next.state = UpdateState::Available;
                next.available_version = Some(manifest.version);
                next.notes = manifest
                    .notes
                    .as_deref()
                    .and_then(|notes| sanitize_release_notes(Some(notes)));
                next.published_at = manifest.published_at;
                info!(version = ?next.available_version, "Lyra update available");
            }
            Ok(_) => {
                next.state = UpdateState::UpToDate;
                next.available_version = None;
                next.notes = None;
                next.published_at = None;
            }
            Err(error) => {
                next.state = UpdateState::Error;
                next.error = Some(error.to_string());
                warn!(error = %error, "Lyra update check failed");
            }
        }
        let published = self.publish(next);
        self.operation_in_flight.store(false, Ordering::Release);
        published
    }

    #[allow(dead_code)]
    pub async fn install_update(
        &self,
        session_state: crate::domain::DictationState,
    ) -> UpdateStatus {
        if let Err(status) = self.reserve_installation(session_state) {
            return status;
        }
        self.install_reserved().await
    }

    pub fn reserve_installation(
        &self,
        session_state: crate::domain::DictationState,
    ) -> Result<(), UpdateStatus> {
        if matches!(
            session_state,
            crate::domain::DictationState::Listening | crate::domain::DictationState::Processing
        ) {
            return Err(self.publish_error("Finish dictation before installing an update"));
        }

        if self
            .installation_gate
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(self.publish_error("An update installation is already in progress"));
        }

        if self.status().available_version.is_none() {
            self.installation_gate.store(false, Ordering::Release);
            return Err(self.publish_error("No update is ready to install"));
        }
        Ok(())
    }

    pub async fn install_reserved(&self) -> UpdateStatus {
        let version = self.status().available_version;
        let Some(version) = version else {
            self.installation_gate.store(false, Ordering::Release);
            return self.publish_error("No update is ready to install");
        };
        if self
            .operation_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            self.installation_gate.store(false, Ordering::Release);
            return self.status();
        }

        let mut downloading = self.status();
        downloading.state = UpdateState::Downloading;
        downloading.progress = Some(0.0);
        downloading.error = None;
        self.publish(downloading);

        let manager = self.clone();
        let progress: UpdateProgressCallback = Arc::new(move |event| {
            let mut status = manager.status();
            status.state = event.phase;
            status.progress = event.total_bytes.map(|total| {
                if total == 0 {
                    0.0
                } else {
                    (event.downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
                }
            });
            manager.publish(status);
        });

        let result = self.provider.install(version, progress).await;
        let status = match result {
            Ok(()) => {
                let mut installing = self.status();
                installing.state = UpdateState::Installing;
                installing.progress = Some(1.0);
                self.publish(installing)
            }
            Err(error) => {
                self.installation_gate.store(false, Ordering::Release);
                self.publish_error(&error.to_string())
            }
        };
        self.operation_in_flight.store(false, Ordering::Release);
        status
    }

    pub fn installation_blocks_sessions(&self) -> bool {
        self.installation_gate.load(Ordering::Acquire)
    }

    pub fn defer_update(&self) -> UpdateStatus {
        if let Some(version) = self.status().available_version {
            if let Ok(mut deferred) = self.deferred_version.lock() {
                *deferred = Some(version);
            }
        }
        self.status()
    }

    fn publish_error(&self, message: &str) -> UpdateStatus {
        let mut status = self.status();
        status.state = UpdateState::Error;
        status.progress = None;
        status.error = Some(message.to_owned());
        self.publish(status)
    }

    fn publish(&self, status: UpdateStatus) -> UpdateStatus {
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        self.sink.emit(&status);
        status
    }
}

pub struct TauriUpdateProvider {
    app: AppHandle,
}

impl TauriUpdateProvider {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl UpdateProvider for TauriUpdateProvider {
    fn check(&self) -> UpdateFuture<Option<UpdateManifest>> {
        let app = self.app.clone();
        Box::pin(async move {
            let updater = app
                .updater()
                .map_err(|error| UpdateError::Provider(error.to_string()))?;
            let update = updater
                .check()
                .await
                .map_err(|error| UpdateError::Provider(error.to_string()))?;
            Ok(update.and_then(|update| {
                if !is_stable_version(&update.version) {
                    return None;
                }
                Some(UpdateManifest {
                    version: update.version,
                    notes: sanitize_release_notes(update.body.as_deref()),
                    published_at: update.date.map(|date| date.to_string()),
                })
            }))
        })
    }

    fn install(&self, version: String, progress: UpdateProgressCallback) -> UpdateFuture<()> {
        let app = self.app.clone();
        Box::pin(async move {
            let updater = app
                .updater()
                .map_err(|error| UpdateError::Provider(error.to_string()))?;
            let update = updater
                .check()
                .await
                .map_err(|error| UpdateError::Provider(error.to_string()))?
                .ok_or(UpdateError::NoUpdate)?;
            if update.version != version {
                return Err(UpdateError::VersionChanged {
                    expected: version,
                    actual: update.version,
                });
            }
            if !is_stable_version(&update.version) {
                return Err(UpdateError::UnstableRelease(update.version));
            }

            let downloaded_bytes = Arc::new(AtomicU64::new(0));
            let progress_download = progress.clone();
            let progress_install = progress.clone();
            let downloaded_for_finish = downloaded_bytes.clone();
            update
                .download_and_install(
                    move |chunk, total| {
                        let downloaded = downloaded_bytes
                            .fetch_add(chunk as u64, Ordering::AcqRel)
                            .saturating_add(chunk as u64);
                        progress_download(UpdateProgress {
                            phase: UpdateState::Downloading,
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                        });
                    },
                    move || {
                        progress_install(UpdateProgress {
                            phase: UpdateState::Installing,
                            downloaded_bytes: downloaded_for_finish.load(Ordering::Acquire),
                            total_bytes: None,
                        });
                    },
                )
                .await
                .map_err(|error| UpdateError::Provider(error.to_string()))
        })
    }
}

struct TauriUpdateEventSink {
    app: AppHandle,
}

impl TauriUpdateEventSink {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl UpdateEventSink for TauriUpdateEventSink {
    fn emit(&self, status: &UpdateStatus) {
        let _ = tauri::Emitter::emit(&self.app, UPDATE_STATUS_EVENT, status);
        crate::tray::update_tray_status(&self.app, status);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("update provider failed: {0}")]
    Provider(String),
    #[error("no update is available")]
    NoUpdate,
    #[error("the available update changed from {expected} to {actual}")]
    VersionChanged { expected: String, actual: String },
    #[error("unstable release ignored: {0}")]
    UnstableRelease(String),
}

pub fn is_stable_version(version: &str) -> bool {
    Version::parse(version.trim_start_matches('v'))
        .map(|version| version.pre.is_empty())
        .unwrap_or(false)
}

pub fn is_newer_stable_version(current: &str, candidate: &str) -> bool {
    let Ok(current) = Version::parse(current.trim_start_matches('v')) else {
        return false;
    };
    let Ok(candidate_version) = Version::parse(candidate.trim_start_matches('v')) else {
        return false;
    };
    candidate_version.pre.is_empty() && candidate_version > current
}

pub fn sanitize_release_notes(notes: Option<&str>) -> Option<String> {
    let notes = notes?;
    let mut sanitized = String::with_capacity(notes.len().min(720));
    let mut inside_tag = false;
    for character in notes.chars() {
        match character {
            '<' => inside_tag = true,
            '>' if inside_tag => inside_tag = false,
            '\r' => {}
            '\n' | '\t' => sanitized.push(' '),
            _ if inside_tag => {}
            _ if character.is_control() && character != '\n' && character != '\t' => {}
            _ => sanitized.push(character),
        }
        if sanitized.chars().count() >= 720 {
            break;
        }
    }
    let sanitized = sanitized.trim().to_owned();
    (!sanitized.is_empty()).then_some(sanitized)
}

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct TestSink {
        emissions: Arc<Mutex<Vec<UpdateStatus>>>,
    }

    impl UpdateEventSink for TestSink {
        fn emit(&self, status: &UpdateStatus) {
            self.emissions.lock().unwrap().push(status.clone());
        }
    }

    struct FakeProvider {
        checks: AtomicUsize,
    }

    impl UpdateProvider for FakeProvider {
        fn check(&self) -> UpdateFuture<Option<UpdateManifest>> {
            self.checks.fetch_add(1, Ordering::AcqRel);
            Box::pin(async {
                Ok(Some(UpdateManifest {
                    version: "0.3.0".to_owned(),
                    notes: Some("A small <b>safe</b> update".to_owned()),
                    published_at: Some("today".to_owned()),
                }))
            })
        }

        fn install(&self, _version: String, progress: UpdateProgressCallback) -> UpdateFuture<()> {
            Box::pin(async move {
                progress(UpdateProgress {
                    phase: UpdateState::Downloading,
                    downloaded_bytes: 5,
                    total_bytes: Some(10),
                });
                progress(UpdateProgress {
                    phase: UpdateState::Installing,
                    downloaded_bytes: 10,
                    total_bytes: Some(10),
                });
                Ok(())
            })
        }
    }

    #[test]
    fn stable_version_comparison_rejects_prereleases_and_downgrades() {
        assert!(is_newer_stable_version("0.2.0", "v0.3.0"));
        assert!(!is_newer_stable_version("0.2.0", "0.3.0-rc.1"));
        assert!(!is_newer_stable_version("0.2.0", "0.1.9"));
    }

    #[test]
    fn release_notes_are_plain_text_and_bounded() {
        assert_eq!(
            sanitize_release_notes(Some(" <b>hello</b>\r\nworld ")).as_deref(),
            Some("hello world")
        );
        assert!(sanitize_release_notes(Some("\u{0}\u{1}")).is_none());
    }

    #[test]
    fn fake_provider_drives_available_and_installing_states() {
        let emissions = Arc::new(Mutex::new(Vec::new()));
        let manager = UpdateManager::with_provider(
            Arc::new(FakeProvider {
                checks: AtomicUsize::new(0),
            }),
            Arc::new(TestSink {
                emissions: emissions.clone(),
            }),
            "0.2.0",
        );
        let status = tauri::async_runtime::block_on(manager.check_for_updates());
        assert_eq!(status.state, UpdateState::Available);
        assert_eq!(status.notes.as_deref(), Some("A small safe update"));
        let status = tauri::async_runtime::block_on(
            manager.install_update(crate::domain::DictationState::Idle),
        );
        assert_eq!(status.state, UpdateState::Installing);
        assert!(manager.installation_blocks_sessions());
        assert!(emissions.lock().unwrap().len() >= 4);
    }

    struct FailingProvider {
        fail_check: bool,
    }

    impl UpdateProvider for FailingProvider {
        fn check(&self) -> UpdateFuture<Option<UpdateManifest>> {
            if self.fail_check {
                return Box::pin(async {
                    Err(UpdateError::Provider(
                        "offline fixture endpoint unavailable".to_owned(),
                    ))
                });
            }
            Box::pin(async {
                Ok(Some(UpdateManifest {
                    version: "0.3.0".to_owned(),
                    notes: None,
                    published_at: None,
                }))
            })
        }

        fn install(&self, _version: String, _progress: UpdateProgressCallback) -> UpdateFuture<()> {
            Box::pin(async {
                Err(UpdateError::Provider(
                    "fixture installer refused to replace the application".to_owned(),
                ))
            })
        }
    }

    #[test]
    fn active_dictation_blocks_install() {
        let manager = UpdateManager::with_provider(
            Arc::new(FakeProvider {
                checks: AtomicUsize::new(0),
            }),
            Arc::new(TestSink {
                emissions: Arc::new(Mutex::new(Vec::new())),
            }),
            "0.2.0",
        );
        let status = tauri::async_runtime::block_on(
            manager.install_update(crate::domain::DictationState::Listening),
        );
        assert_eq!(status.state, UpdateState::Error);
        assert!(status.error.unwrap().contains("Finish dictation"));
    }

    #[test]
    fn offline_check_and_failed_install_are_recoverable() {
        let offline = UpdateManager::with_provider(
            Arc::new(FailingProvider { fail_check: true }),
            Arc::new(TestSink {
                emissions: Arc::new(Mutex::new(Vec::new())),
            }),
            "0.2.0",
        );
        let status = tauri::async_runtime::block_on(offline.check_for_updates());
        assert_eq!(status.state, UpdateState::Error);
        assert!(!offline.installation_blocks_sessions());

        let failed_install = UpdateManager::with_provider(
            Arc::new(FailingProvider { fail_check: false }),
            Arc::new(TestSink {
                emissions: Arc::new(Mutex::new(Vec::new())),
            }),
            "0.2.0",
        );
        let available = tauri::async_runtime::block_on(failed_install.check_for_updates());
        assert_eq!(available.state, UpdateState::Available);
        let status = tauri::async_runtime::block_on(
            failed_install.install_update(crate::domain::DictationState::Idle),
        );
        assert_eq!(status.state, UpdateState::Error);
        assert!(status.error.unwrap().contains("refused"));
        assert!(!failed_install.installation_blocks_sessions());
    }
}

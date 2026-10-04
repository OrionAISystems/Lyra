use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use tracing::warn;
use uuid::Uuid;

use crate::{settings::TranscriptHistoryRetention, usage::timestamp_ms};

const ITEM_SCHEMA_VERSION: u32 = 1;
const MAX_ITEM_FILE_BYTES: u64 = 4 * 1024 * 1024;
const DAY_MS: u64 = 86_400_000;
const MAX_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedItemSummary {
    pub id: String,
    pub timestamp_ms: u64,
    pub duration_ms: u64,
    pub word_count: u64,
    pub target_app: Option<String>,
    pub target_executable: Option<String>,
    pub engine: String,
    pub backend: String,
    pub model: String,
    pub latency_ms: u64,
    pub paste_outcome: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedItemPage {
    pub items: Vec<SavedItemSummary>,
    pub total: usize,
    pub offset: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedItemFile {
    schema_version: u32,
    summary: SavedItemSummary,
    encrypted_text: String,
}

#[derive(Debug, Clone)]
pub struct SavedItemStore {
    root: PathBuf,
}

impl SavedItemStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn save_history(
        &self,
        mut summary: SavedItemSummary,
        text: &str,
        retention: TranscriptHistoryRetention,
    ) -> io::Result<Option<SavedItemSummary>> {
        if !retention.is_enabled() {
            return Ok(None);
        }
        self.prune_history(retention, retention)?;
        summary.id = Uuid::new_v4().to_string();
        self.write_item("history", summary.clone(), text)?;
        Ok(Some(summary))
    }

    pub fn create_scratchpad(
        &self,
        mut summary: SavedItemSummary,
        text: &str,
    ) -> io::Result<SavedItemSummary> {
        summary.id = Uuid::new_v4().to_string();
        summary.pinned = false;
        self.write_item("scratchpad", summary.clone(), text)?;
        Ok(summary)
    }

    pub fn list_history_page(
        &self,
        retention: TranscriptHistoryRetention,
        last_retention: TranscriptHistoryRetention,
        offset: usize,
        limit: usize,
    ) -> io::Result<SavedItemPage> {
        self.prune_history(retention, last_retention)?;
        self.list_collection_page("history", offset, limit)
    }

    pub fn list_scratchpad_page(&self, offset: usize, limit: usize) -> io::Result<SavedItemPage> {
        self.list_collection_page("scratchpad", offset, limit)
    }

    pub fn history_text(&self, id: &str) -> io::Result<(SavedItemSummary, String)> {
        self.read_item("history", id)
    }

    pub fn scratchpad_text(&self, id: &str) -> io::Result<(SavedItemSummary, String)> {
        self.read_item("scratchpad", id)
    }

    pub fn delete_history(&self, id: &str) -> io::Result<()> {
        self.delete_item("history", id)
    }

    pub fn delete_scratchpad(&self, id: &str) -> io::Result<()> {
        self.delete_item("scratchpad", id)
    }

    pub fn clear_history(&self) -> io::Result<()> {
        self.clear_collection("history")
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> io::Result<SavedItemSummary> {
        let (mut summary, text) = self.read_item("scratchpad", id)?;
        summary.pinned = pinned;
        self.write_item("scratchpad", summary.clone(), &text)?;
        Ok(summary)
    }

    pub fn promote_history(&self, id: &str) -> io::Result<SavedItemSummary> {
        let (mut summary, text) = self.history_text(id)?;
        summary.id.clear();
        summary.timestamp_ms = timestamp_ms();
        summary.pinned = false;
        self.create_scratchpad(summary, &text)
    }

    pub fn prune_history(
        &self,
        retention: TranscriptHistoryRetention,
        last_retention: TranscriptHistoryRetention,
    ) -> io::Result<usize> {
        let effective = if retention.is_enabled() {
            retention
        } else {
            last_retention
        };
        let Some(days) = effective.days() else {
            return Ok(0);
        };
        if days == u64::MAX {
            return Ok(0);
        }
        let cutoff = timestamp_ms().saturating_sub(days.saturating_mul(DAY_MS));
        self.prune_collection_before("history", cutoff)
    }

    fn write_item(
        &self,
        collection: &str,
        summary: SavedItemSummary,
        text: &str,
    ) -> io::Result<()> {
        if text.len() > MAX_ITEM_FILE_BYTES as usize / 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "saved text exceeds the local storage limit",
            ));
        }
        let encrypted = protect_text(text.as_bytes())?;
        let record = SavedItemFile {
            schema_version: ITEM_SCHEMA_VERSION,
            summary,
            encrypted_text: STANDARD.encode(encrypted),
        };
        let bytes = serde_json::to_vec(&record).map_err(io::Error::other)?;
        let path = self.item_path(collection, &record.summary.id)?;
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("saved item path has no parent"))?;
        fs::create_dir_all(parent)?;
        atomic_write(&path, &bytes)
    }

    fn list_collection_page(
        &self,
        collection: &str,
        offset: usize,
        limit: usize,
    ) -> io::Result<SavedItemPage> {
        let dir = self.collection_path(collection)?;
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(SavedItemPage {
                    items: Vec::new(),
                    total: 0,
                    offset,
                    has_more: false,
                })
            }
            Err(error) => return Err(error),
        };
        let mut summaries = Vec::new();
        for entry in entries {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "json")
            {
                continue;
            }
            match read_record(&entry.path()) {
                Ok(record) => summaries.push(record.summary),
                Err(error) => {
                    warn!("saved item metadata could not be read; skipping one item: {error}")
                }
            }
        }
        summaries.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(|| right.timestamp_ms.cmp(&left.timestamp_ms))
        });
        let total = summaries.len();
        let start = offset.min(total);
        let end = start
            .saturating_add(limit.clamp(1, MAX_PAGE_SIZE))
            .min(total);
        Ok(SavedItemPage {
            items: summaries.drain(start..end).collect(),
            total,
            offset: start,
            has_more: end < total,
        })
    }

    fn read_item(&self, collection: &str, id: &str) -> io::Result<(SavedItemSummary, String)> {
        let path = self.item_path(collection, id)?;
        let record = read_record(&path)?;
        let encrypted = STANDARD
            .decode(record.encrypted_text)
            .map_err(io::Error::other)?;
        let plaintext = unprotect_text(&encrypted)?;
        let text = String::from_utf8(plaintext)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "saved text is not UTF-8"))?;
        Ok((record.summary, text))
    }

    fn delete_item(&self, collection: &str, id: &str) -> io::Result<()> {
        let path = self.item_path(collection, id)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Err(error),
            Err(error) => Err(error),
        }
    }

    fn clear_collection(&self, collection: &str) -> io::Result<()> {
        let dir = self.collection_path(collection)?;
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    let path = entry?.path();
                    if path.is_file() {
                        fs::remove_file(path)?;
                    }
                }
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn prune_collection_before(&self, collection: &str, cutoff: u64) -> io::Result<usize> {
        let dir = self.collection_path(collection)?;
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
        };
        let mut removed = 0;
        for entry in entries {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            match read_record(&path) {
                Ok(record) if record.summary.timestamp_ms < cutoff => {
                    fs::remove_file(path)?;
                    removed += 1;
                }
                Ok(_) => {}
                Err(error) => {
                    warn!("saved item could not be checked for expiry; preserving it: {error}")
                }
            }
        }
        Ok(removed)
    }

    fn collection_path(&self, collection: &str) -> io::Result<PathBuf> {
        match collection {
            "history" | "scratchpad" => Ok(self.root.join(collection)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid saved item collection",
            )),
        }
    }

    fn item_path(&self, collection: &str, id: &str) -> io::Result<PathBuf> {
        let id = Uuid::parse_str(id)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid saved item id"))?;
        Ok(self.collection_path(collection)?.join(format!("{id}.json")))
    }
}

fn read_record(path: &Path) -> io::Result<SavedItemFile> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_ITEM_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ITEM_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved item exceeds the local file limit",
        ));
    }
    let record: SavedItemFile = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if record.schema_version != ITEM_SCHEMA_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved item version is unsupported",
        ));
    }
    Ok(record)
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(source, destination)
}

#[cfg(windows)]
fn protect_text(text: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(text.len()).map_err(io::Error::other)?,
        pbData: text.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    let success = unsafe {
        CryptProtectData(
            &input,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    let protected =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(protected)
}

#[cfg(windows)]
fn unprotect_text(ciphertext: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(ciphertext.len()).map_err(io::Error::other)?,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    let success = unsafe {
        CryptUnprotectData(
            &input,
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    let plaintext =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(plaintext)
}

#[cfg(not(windows))]
fn protect_text(_text: &[u8]) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows user-bound text protection is unavailable on this platform",
    ))
}

#[cfg(not(windows))]
fn unprotect_text(_ciphertext: &[u8]) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows user-bound text protection is unavailable on this platform",
    ))
}

pub fn display_app_name(executable: Option<&str>) -> Option<String> {
    let executable = executable?.trim();
    let stem = Path::new(executable).file_stem()?.to_string_lossy();
    let label = match stem.to_ascii_lowercase().as_str() {
        "code" => "VS Code".to_owned(),
        "chatgpt" => "ChatGPT".to_owned(),
        "codex" => "Codex".to_owned(),
        "notepad" => "Notepad".to_owned(),
        "notepad++" => "Notepad++".to_owned(),
        "obsidian" => "Obsidian".to_owned(),
        "devenv" => "Visual Studio".to_owned(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => return None,
            }
        }
    };
    Some(label)
}

pub fn local_root(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(windows)]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("Lyra")
            .join("saved-items");
    }
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("app-data"))
        .join("Lyra")
        .join("saved-items")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_names_use_friendly_labels_and_drop_path_information() {
        assert_eq!(
            display_app_name(Some("code.exe")),
            Some("VS Code".to_owned())
        );
        assert_eq!(
            display_app_name(Some("C:\\Users\\Person\\notepad.exe")),
            Some("Notepad".to_owned())
        );
        assert_eq!(display_app_name(None), None);
    }

    #[test]
    fn retention_has_finite_periods_and_forever() {
        assert_eq!(TranscriptHistoryRetention::SevenDays.days(), Some(7));
        assert_eq!(TranscriptHistoryRetention::ThirtyDays.days(), Some(30));
        assert_eq!(TranscriptHistoryRetention::NinetyDays.days(), Some(90));
        assert_eq!(TranscriptHistoryRetention::OneYear.days(), Some(365));
        assert_eq!(TranscriptHistoryRetention::Forever.days(), Some(u64::MAX));
        assert_eq!(TranscriptHistoryRetention::Off.days(), None);
    }

    #[test]
    fn history_off_does_not_create_or_write_a_store() {
        let root = std::env::temp_dir().join(format!("lyra-history-off-{}", Uuid::new_v4()));
        let store = SavedItemStore::new(root.clone());

        assert!(store
            .save_history(
                summary(timestamp_ms()),
                "must not persist",
                TranscriptHistoryRetention::Off,
            )
            .unwrap()
            .is_none());
        assert!(!root.exists());
    }

    #[cfg(windows)]
    #[test]
    fn saved_text_is_encrypted_and_collections_keep_independent_lifecycles() {
        let root = std::env::temp_dir().join(format!("lyra-saved-{}", Uuid::new_v4()));
        let store = SavedItemStore::new(root.clone());
        let now = timestamp_ms();
        let old = store
            .save_history(
                summary(now.saturating_sub(45 * DAY_MS)),
                "sensitive history transcript",
                TranscriptHistoryRetention::Forever,
            )
            .unwrap()
            .unwrap();
        let middle = store
            .save_history(
                summary(now.saturating_sub(15 * DAY_MS)),
                "another private transcript",
                TranscriptHistoryRetention::Forever,
            )
            .unwrap()
            .unwrap();
        let newest = store
            .save_history(
                summary(now.saturating_sub(DAY_MS)),
                "recent private transcript",
                TranscriptHistoryRetention::Forever,
            )
            .unwrap()
            .unwrap();

        let encoded_file =
            fs::read_to_string(store.item_path("history", &newest.id).unwrap()).unwrap();
        assert!(!encoded_file.contains("recent private transcript"));
        assert_eq!(
            store.history_text(&newest.id).unwrap().1,
            "recent private transcript"
        );

        let first_page = store
            .list_history_page(
                TranscriptHistoryRetention::Forever,
                TranscriptHistoryRetention::ThirtyDays,
                0,
                1,
            )
            .unwrap();
        assert_eq!(first_page.items.len(), 1);
        assert_eq!(first_page.total, 3);
        assert!(first_page.has_more);
        assert_eq!(first_page.items[0].id, newest.id);

        assert_eq!(
            store
                .prune_history(
                    TranscriptHistoryRetention::Off,
                    TranscriptHistoryRetention::ThirtyDays,
                )
                .unwrap(),
            1
        );
        assert!(store.history_text(&old.id).is_err());
        assert!(store.history_text(&middle.id).is_ok());

        let older_note = store
            .create_scratchpad(
                summary(now.saturating_sub(60 * DAY_MS)),
                "intentional thought",
            )
            .unwrap();
        store.set_pinned(&older_note.id, true).unwrap();
        let promoted = store.promote_history(&middle.id).unwrap();
        assert!(promoted.timestamp_ms >= now);
        let scratchpad_page = store.list_scratchpad_page(0, 1).unwrap();
        assert_eq!(scratchpad_page.total, 2);
        assert!(scratchpad_page.items[0].pinned);
        assert_eq!(scratchpad_page.items[0].id, older_note.id);

        assert_eq!(
            store
                .prune_history(
                    TranscriptHistoryRetention::SevenDays,
                    TranscriptHistoryRetention::SevenDays,
                )
                .unwrap(),
            1
        );
        assert!(store.history_text(&middle.id).is_err());
        assert!(store.history_text(&newest.id).is_ok());

        store.clear_history().unwrap();
        assert!(store
            .list_history_page(
                TranscriptHistoryRetention::Forever,
                TranscriptHistoryRetention::ThirtyDays,
                0,
                10
            )
            .unwrap()
            .items
            .is_empty());
        assert_eq!(
            store.scratchpad_text(&older_note.id).unwrap().1,
            "intentional thought"
        );
        assert_eq!(
            store.scratchpad_text(&promoted.id).unwrap().1,
            "another private transcript"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn corrupt_encrypted_payload_fails_closed() {
        let root = std::env::temp_dir().join(format!("lyra-corrupt-{}", Uuid::new_v4()));
        let store = SavedItemStore::new(root.clone());
        let item = store
            .save_history(
                summary(timestamp_ms()),
                "private text",
                TranscriptHistoryRetention::Forever,
            )
            .unwrap()
            .unwrap();
        let path = store.item_path("history", &item.id).unwrap();
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        record["encryptedText"] = serde_json::Value::String(STANDARD.encode(b"corrupted"));
        fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();

        assert!(store.history_text(&item.id).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    fn summary(timestamp_ms: u64) -> SavedItemSummary {
        SavedItemSummary {
            id: String::new(),
            timestamp_ms,
            duration_ms: 42_000,
            word_count: 143,
            target_app: Some("ChatGPT".to_owned()),
            target_executable: Some("ChatGPT.exe".to_owned()),
            engine: "parakeet_tdt".to_owned(),
            backend: "local_parakeet".to_owned(),
            model: "Parakeet TDT".to_owned(),
            latency_ms: 190,
            paste_outcome: "success".to_owned(),
            pinned: false,
        }
    }
}

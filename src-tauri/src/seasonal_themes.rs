use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign_verify::{PublicKey, Signature};
use png::{Decoder, Limits};
use reqwest::{
    blocking::{Client, Response},
    redirect::Policy,
    Url,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use time::{format_description::well_known::Rfc3339, Date, Month, OffsetDateTime, Time, UtcOffset};
use tracing::{info, warn};

use crate::settings::Settings;

const CATALOG_URL: &str =
    "https://raw.githubusercontent.com/OrionAISystems/Lyra-Releases/main/themes/catalog.json";
const CATALOG_DISCRIMINATOR: &str = "lyra-seasonal-themes";
const CATALOG_SCHEMA_VERSION: u32 = 1;
const ENGINE_VERSION: u32 = 1;
const MAX_CATALOG_BYTES: usize = 64 * 1024;
const MAX_ENVELOPE_BYTES: usize = 128 * 1024;
const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;
const MAX_DECODED_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 1024;
const MAX_CACHED_REMOTE_THEMES: usize = 2;
const MAX_CACHE_IMAGE_BYTES: usize = MAX_TOTAL_IMAGE_BYTES * MAX_CACHED_REMOTE_THEMES;
const MAX_SNAPSHOT_SCAN: usize = 32;
const REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);
const FAILURE_RETRY_INTERVAL: Duration = Duration::from_secs(60 * 60);
const CLOCK_POLL_INTERVAL: Duration = Duration::from_secs(30);
const THEME_CHANGED_EVENT: &str = "lyra://theme-changed";
const THEME_ROOT_DIR: &str = "seasonal-themes";

const TAURI_CONFIG: &str = include_str!("../tauri.conf.json");

const ARTWORK_SLOTS: [ArtworkSlot; 8] = [
    ArtworkSlot::Idle,
    ArtworkSlot::Listening,
    ArtworkSlot::Processing,
    ArtworkSlot::Success,
    ArtworkSlot::Error,
    ArtworkSlot::Bar,
    ArtworkSlot::Empty,
    ArtworkSlot::Ambience,
];

const SURFACE_TOKENS: [&str; 8] = [
    "bg", "panel", "panel2", "panel3", "chrome", "sidebar", "surface", "control",
];
const FOREGROUND_TOKENS: [&str; 4] = ["text", "muted", "accent", "accent2"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtworkSlot {
    Idle,
    Listening,
    Processing,
    Success,
    Error,
    Bar,
    Empty,
    Ambience,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemePalette {
    pub bg: String,
    pub panel: String,
    pub panel2: String,
    pub panel3: String,
    pub line: String,
    pub text: String,
    pub muted: String,
    pub chrome: String,
    pub sidebar: String,
    pub surface: String,
    pub control: String,
    pub accent: String,
    pub accent2: String,
    pub glow: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePalettes {
    pub dark: ThemePalette,
    pub light: ThemePalette,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeContent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub start_month_day: String,
    pub end_month_day: String,
    pub revision: u64,
    pub palettes: ThemePalettes,
    pub artwork: BTreeMap<ArtworkSlot, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSource {
    Remote,
    Cache,
    Bundled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeState {
    Disabled,
    Active,
    Upcoming,
    Fallback,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableTheme {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub start_month_day: String,
    pub end_month_day: String,
    pub source: ThemeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeStatus {
    pub enabled: bool,
    pub motion_enabled: bool,
    pub state: ThemeState,
    pub theme_id: Option<String>,
    pub catalog_revision: Option<u64>,
    pub next_change_at: Option<String>,
    pub refreshed_at: Option<String>,
    pub error: Option<String>,
    pub source: Option<ThemeSource>,
    pub available_themes: Vec<AvailableTheme>,
    pub content: Option<ThemeContent>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SignedCatalogEnvelope {
    payload: String,
    signature: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThemeCatalogPayload {
    discriminator: String,
    schema_version: u32,
    engine_version: u32,
    revision: u64,
    themes: Vec<RemoteThemeDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoteThemeDescriptor {
    id: String,
    name: String,
    description: String,
    start_month_day: String,
    end_month_day: String,
    revision: u64,
    palettes: ThemePalettes,
    artwork: BTreeMap<ArtworkSlot, RemoteArtwork>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoteArtwork {
    url: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundledThemeDescriptor {
    id: String,
    name: String,
    description: String,
    start_month_day: String,
    end_month_day: String,
    revision: u64,
    palettes: ThemePalettes,
    artwork: BTreeMap<ArtworkSlot, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CachePointer {
    high_water_revision: u64,
    active_snapshot: String,
    active_payload_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotManifest {
    selected_theme_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeErrorCode {
    CatalogUnavailable,
    CatalogInvalid,
    RollbackDetected,
    AssetsUnavailable,
    CacheUnavailable,
}

impl ThemeErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::CatalogUnavailable => "catalog-unavailable",
            Self::CatalogInvalid => "catalog-invalid",
            Self::RollbackDetected => "catalog-rollback-rejected",
            Self::AssetsUnavailable => "assets-unavailable",
            Self::CacheUnavailable => "cache-unavailable",
        }
    }
}

#[derive(Default)]
struct ThemeRuntimeState {
    catalog: Option<ThemeCatalogPayload>,
    remote_content: BTreeMap<String, ThemeContent>,
    revision: Option<u64>,
    cache_source: bool,
    refreshed_at: Option<OffsetDateTime>,
    last_error: Option<ThemeErrorCode>,
}

struct ThemeManagerInner {
    app: AppHandle,
    root: PathBuf,
    enabled: AtomicBool,
    motion_enabled: AtomicBool,
    state: Mutex<ThemeRuntimeState>,
    emission_gate: Mutex<()>,
    last_emitted: Mutex<Option<ThemeStatus>>,
    refresh_gate: Mutex<()>,
    refresh_timer: Mutex<RefreshTimer>,
    started: AtomicBool,
}

#[derive(Clone)]
pub struct SeasonalThemeManager {
    inner: Arc<ThemeManagerInner>,
}

#[derive(Default)]
struct RefreshTimer {
    last_attempt: Option<Instant>,
    failed: bool,
}

#[derive(Clone, Copy)]
struct ClockObservation {
    utc_timestamp: i64,
    local_date: Date,
    offset_seconds: i32,
    monotonic: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AnnualDate {
    month: u8,
    day: u8,
    day_index: u16,
}

impl AnnualDate {
    fn parse(value: &str) -> Result<Self, ThemeErrorCode> {
        let bytes = value.as_bytes();
        if bytes.len() != 5
            || bytes[2] != b'-'
            || !bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 2 || b.is_ascii_digit())
        {
            return Err(ThemeErrorCode::CatalogInvalid);
        }
        let month = value[0..2]
            .parse::<u8>()
            .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
        let day = value[3..5]
            .parse::<u8>()
            .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
        let month_value = Month::try_from(month).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
        let date = Date::from_calendar_date(2000, month_value, day)
            .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
        Ok(Self {
            month,
            day,
            day_index: date.ordinal() as u16 - 1,
        })
    }

    fn key(self) -> (u8, u8) {
        (self.month, self.day)
    }
}

fn validate_theme_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.as_bytes()[value.len() - 1].is_ascii_lowercase()
}

fn validate_text(value: &str, max_chars: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn color_rgb(value: &str) -> Option<[u8; 3]> {
    let bytes = value.strip_prefix('#')?.as_bytes();
    if bytes.len() != 6 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some([
        u8::from_str_radix(&value[1..3], 16).ok()?,
        u8::from_str_radix(&value[3..5], 16).ok()?,
        u8::from_str_radix(&value[5..7], 16).ok()?,
    ])
}

fn relative_luminance(rgb: [u8; 3]) -> f64 {
    let linear = |channel: u8| {
        let value = channel as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f64 {
    let a = relative_luminance(first);
    let b = relative_luminance(second);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn validate_palette(palette: &ThemePalette) -> Result<(), ThemeErrorCode> {
    let tokens = [
        ("bg", palette.bg.as_str()),
        ("panel", palette.panel.as_str()),
        ("panel2", palette.panel2.as_str()),
        ("panel3", palette.panel3.as_str()),
        ("line", palette.line.as_str()),
        ("text", palette.text.as_str()),
        ("muted", palette.muted.as_str()),
        ("chrome", palette.chrome.as_str()),
        ("sidebar", palette.sidebar.as_str()),
        ("surface", palette.surface.as_str()),
        ("control", palette.control.as_str()),
        ("accent", palette.accent.as_str()),
        ("accent2", palette.accent2.as_str()),
        ("glow", palette.glow.as_str()),
    ];
    let mut parsed = HashMap::new();
    for (name, value) in tokens {
        parsed.insert(
            name,
            color_rgb(value).ok_or(ThemeErrorCode::CatalogInvalid)?,
        );
    }
    for foreground in FOREGROUND_TOKENS {
        let foreground = parsed[foreground];
        for surface in SURFACE_TOKENS {
            if contrast_ratio(foreground, parsed[surface]) < 4.5 {
                return Err(ThemeErrorCode::CatalogInvalid);
            }
        }
    }
    Ok(())
}

fn validate_palettes(palettes: &ThemePalettes) -> Result<(), ThemeErrorCode> {
    validate_palette(&palettes.dark)?;
    validate_palette(&palettes.light)
}

fn annual_segments(start: AnnualDate, end: AnnualDate) -> Vec<(u16, u16)> {
    if start.key() < end.key() {
        vec![(start.day_index, end.day_index)]
    } else {
        vec![(start.day_index, 366), (0, end.day_index)]
    }
}

fn ranges_overlap(
    first_start: AnnualDate,
    first_end: AnnualDate,
    second_start: AnnualDate,
    second_end: AnnualDate,
) -> bool {
    annual_segments(first_start, first_end)
        .iter()
        .any(|(first_a, first_b)| {
            annual_segments(second_start, second_end)
                .iter()
                .any(|(second_a, second_b)| first_a < second_b && second_a < first_b)
        })
}

fn annual_occurrence_date(annual_date: AnnualDate, year: i32) -> Option<Date> {
    let month = Month::try_from(annual_date.month).ok()?;
    Date::from_calendar_date(year, month, annual_date.day)
        .ok()
        // A recurring February 29 boundary moves to March 1 in non-leap years.
        .or_else(|| {
            (annual_date.month == 2 && annual_date.day == 29)
                .then(|| Date::from_calendar_date(year, Month::March, 1).ok())
                .flatten()
        })
}

fn is_in_annual_range(today: Date, start: AnnualDate, end: AnnualDate) -> bool {
    let Some(start_date) = annual_occurrence_date(start, today.year()) else {
        return false;
    };
    let Some(end_date) = annual_occurrence_date(end, today.year()) else {
        return false;
    };
    if start.key() < end.key() {
        today >= start_date && today < end_date
    } else {
        today >= start_date || today < end_date
    }
}

fn validate_artwork_slots<T>(artwork: &BTreeMap<ArtworkSlot, T>) -> Result<(), ThemeErrorCode> {
    if artwork.len() != ARTWORK_SLOTS.len()
        || ARTWORK_SLOTS.iter().any(|slot| !artwork.contains_key(slot))
    {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    Ok(())
}

fn validate_remote_url(
    value: &str,
    theme_id: &str,
    theme_revision: u64,
) -> Result<(), ThemeErrorCode> {
    let url = Url::parse(value).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path().contains('%')
        || url.path().contains('\\')
    {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    let segments = url
        .path_segments()
        .ok_or(ThemeErrorCode::CatalogInvalid)?
        .collect::<Vec<_>>();
    if segments.len() != 6
        || segments[0] != "OrionAISystems"
        || segments[1] != "Lyra-Releases"
        || segments[2] != "releases"
        || segments[3] != "download"
        || segments[4] != format!("themes-{theme_id}-v{theme_revision}")
        || segments[5].len() <= 4
        || segments[5].len() > 128
        || !segments[5]
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !segments[5].ends_with(".png")
        || !segments[5]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || segments[5].contains("..")
    {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    Ok(())
}

fn validate_catalog_payload(payload: &[u8]) -> Result<ThemeCatalogPayload, ThemeErrorCode> {
    if payload.is_empty() || payload.len() > MAX_CATALOG_BYTES {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    let catalog: ThemeCatalogPayload =
        serde_json::from_slice(payload).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    if catalog.discriminator != CATALOG_DISCRIMINATOR
        || catalog.schema_version != CATALOG_SCHEMA_VERSION
        || catalog.engine_version != ENGINE_VERSION
        || catalog.revision == 0
        || catalog.themes.is_empty()
        || catalog.themes.len() > 64
    {
        return Err(ThemeErrorCode::CatalogInvalid);
    }

    let mut theme_ids = BTreeSet::new();
    let mut annual_ranges = Vec::new();
    let mut all_assets = BTreeMap::<String, u64>::new();
    for theme in &catalog.themes {
        if !validate_theme_id(&theme.id)
            || !theme_ids.insert(theme.id.as_str())
            || !validate_text(&theme.name, 80)
            || !validate_text(&theme.description, 300)
            || theme.revision == 0
        {
            return Err(ThemeErrorCode::CatalogInvalid);
        }
        validate_palettes(&theme.palettes)?;
        let start = AnnualDate::parse(&theme.start_month_day)?;
        let end = AnnualDate::parse(&theme.end_month_day)?;
        if start.key() == end.key() {
            return Err(ThemeErrorCode::CatalogInvalid);
        }
        for (other_start, other_end) in &annual_ranges {
            if ranges_overlap(start, end, *other_start, *other_end) {
                return Err(ThemeErrorCode::CatalogInvalid);
            }
        }
        annual_ranges.push((start, end));

        validate_artwork_slots(&theme.artwork)?;
        let mut theme_assets = BTreeMap::<String, u64>::new();
        for artwork in theme.artwork.values() {
            validate_remote_url(&artwork.url, &theme.id, theme.revision)?;
            if !(45..=MAX_IMAGE_BYTES as u64).contains(&artwork.size_bytes) {
                return Err(ThemeErrorCode::CatalogInvalid);
            }
            if artwork.sha256.len() != 64
                || !artwork
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(ThemeErrorCode::CatalogInvalid);
            }
            if let Some(previous_size) =
                all_assets.insert(artwork.sha256.clone(), artwork.size_bytes)
            {
                if previous_size != artwork.size_bytes {
                    return Err(ThemeErrorCode::CatalogInvalid);
                }
            }
            theme_assets.insert(artwork.sha256.clone(), artwork.size_bytes);
        }
        if theme_assets
            .values()
            .try_fold(0_u64, |sum, size| sum.checked_add(*size))
            .is_none_or(|total| total > MAX_TOTAL_IMAGE_BYTES as u64)
        {
            return Err(ThemeErrorCode::CatalogInvalid);
        }
    }
    Ok(catalog)
}

fn verify_signed_catalog(
    envelope_bytes: &[u8],
    public_key_text: &str,
) -> Result<(Vec<u8>, ThemeCatalogPayload), ThemeErrorCode> {
    if envelope_bytes.is_empty() || envelope_bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    let envelope: SignedCatalogEnvelope =
        serde_json::from_slice(envelope_bytes).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let payload = STANDARD
        .decode(envelope.payload.as_bytes())
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    if payload.is_empty() || payload.len() > MAX_CATALOG_BYTES {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    let signature_bytes = STANDARD
        .decode(envelope.signature.as_bytes())
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    // The Tauri signer `.sig` file is base64-encoded minisign text. The envelope
    // preserves those exact file bytes in its own base64 field, so unwrap both layers.
    let signature_file_text =
        std::str::from_utf8(&signature_bytes).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let decoded_signature_bytes = STANDARD
        .decode(signature_file_text.trim())
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let signature_text = std::str::from_utf8(&decoded_signature_bytes)
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let public_key =
        PublicKey::decode(public_key_text).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let signature =
        Signature::decode(signature_text).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    public_key
        .verify(&payload, &signature, false)
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let catalog = validate_catalog_payload(&payload)?;
    Ok((payload, catalog))
}

fn pinned_public_key_text() -> Result<String, ThemeErrorCode> {
    let config: serde_json::Value =
        serde_json::from_str(TAURI_CONFIG).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    let encoded_key = config
        .get("plugins")
        .and_then(|plugins| plugins.get("updater"))
        .and_then(|updater| updater.get("pubkey"))
        .and_then(serde_json::Value::as_str)
        .ok_or(ThemeErrorCode::CatalogInvalid)?;
    decode_public_key_file(encoded_key)
}

fn decode_public_key_file(encoded_key: &str) -> Result<String, ThemeErrorCode> {
    let key_bytes = STANDARD
        .decode(encoded_key.trim())
        .map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    String::from_utf8(key_bytes).map_err(|_| ThemeErrorCode::CatalogInvalid)
}

fn validate_png(bytes: &[u8]) -> Result<(u32, u32), ThemeErrorCode> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    if png_has_animation_chunk(bytes) {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_limits(Limits {
        bytes: 4 * 1024 * 1024,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_IMAGE_DIMENSION
        || info.height > MAX_IMAGE_DIMENSION
        || info.animation_control.is_some()
    {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    let width = info.width;
    let height = info.height;
    let decoded_size = reader
        .output_buffer_size()
        .ok_or(ThemeErrorCode::AssetsUnavailable)?;
    if decoded_size > MAX_DECODED_IMAGE_BYTES {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    let mut output = vec![0_u8; decoded_size];
    reader
        .next_frame(&mut output)
        .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
    reader
        .finish()
        .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
    Ok((width, height))
}

fn png_has_animation_chunk(bytes: &[u8]) -> bool {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(PNG_SIGNATURE) {
        return true;
    }
    let mut cursor = PNG_SIGNATURE.len();
    loop {
        let Some(header_end) = cursor.checked_add(8) else {
            return true;
        };
        if header_end > bytes.len() {
            return true;
        }
        let length = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        let Some(chunk_end) = cursor
            .checked_add(12)
            .and_then(|offset| offset.checked_add(length))
        else {
            return true;
        };
        if chunk_end > bytes.len() {
            return true;
        }
        let chunk_type = &bytes[cursor + 4..cursor + 8];
        if chunk_type == b"acTL" || chunk_type == b"fcTL" || chunk_type == b"fdAT" {
            return true;
        }
        cursor = chunk_end;
        if chunk_type == b"IEND" {
            return false;
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_artwork_bytes(metadata: &RemoteArtwork, bytes: &[u8]) -> Result<(), ThemeErrorCode> {
    if bytes.len() as u64 != metadata.size_bytes || sha256_hex(bytes) != metadata.sha256 {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    validate_png(bytes).map(|_| ())
}

const BUNDLED_DESCRIPTOR: &str = include_str!("../../themes/october/theme.json");
const BUNDLED_PUMPKIN_PNG: &[u8] = include_bytes!("../../themes/october/assets/pumpkin.png");
const BUNDLED_LEAF_PNG: &[u8] = include_bytes!("../../themes/october/assets/leaf.png");

fn load_bundled_theme() -> Result<ThemeContent, ThemeErrorCode> {
    let descriptor: BundledThemeDescriptor =
        serde_json::from_str(BUNDLED_DESCRIPTOR).map_err(|_| ThemeErrorCode::CatalogInvalid)?;
    if descriptor.id != "october"
        || descriptor.start_month_day != "10-01"
        || descriptor.end_month_day != "11-01"
        || !validate_theme_id(&descriptor.id)
        || !validate_text(&descriptor.name, 80)
        || !validate_text(&descriptor.description, 300)
        || descriptor.revision == 0
    {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    validate_palettes(&descriptor.palettes)?;
    let start = AnnualDate::parse(&descriptor.start_month_day)?;
    let end = AnnualDate::parse(&descriptor.end_month_day)?;
    if start.key() == end.key() {
        return Err(ThemeErrorCode::CatalogInvalid);
    }
    validate_artwork_slots(&descriptor.artwork)?;

    let mut artwork = BTreeMap::new();
    let mut unique_assets = BTreeSet::new();
    for (slot, relative_path) in descriptor.artwork {
        let bytes = match relative_path.as_str() {
            "assets/pumpkin.png" => BUNDLED_PUMPKIN_PNG,
            "assets/leaf.png" => BUNDLED_LEAF_PNG,
            _ => return Err(ThemeErrorCode::CatalogInvalid),
        };
        validate_png(bytes)?;
        unique_assets.insert(relative_path);
        artwork.insert(
            slot,
            format!("data:image/png;base64,{}", STANDARD.encode(bytes)),
        );
    }
    let total_size = unique_assets.iter().try_fold(0_usize, |sum, path| {
        let bytes = match path.as_str() {
            "assets/pumpkin.png" => BUNDLED_PUMPKIN_PNG,
            "assets/leaf.png" => BUNDLED_LEAF_PNG,
            _ => return None,
        };
        sum.checked_add(bytes.len())
    });
    if total_size.is_none_or(|size| size > MAX_TOTAL_IMAGE_BYTES) {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }

    Ok(ThemeContent {
        id: descriptor.id,
        name: descriptor.name,
        description: descriptor.description,
        start_month_day: descriptor.start_month_day,
        end_month_day: descriptor.end_month_day,
        revision: descriptor.revision,
        palettes: descriptor.palettes,
        artwork,
    })
}

fn bundled_theme() -> Result<ThemeContent, ThemeErrorCode> {
    static BUNDLED: std::sync::OnceLock<Result<ThemeContent, ThemeErrorCode>> =
        std::sync::OnceLock::new();
    BUNDLED.get_or_init(load_bundled_theme).clone()
}

fn content_from_remote(
    descriptor: &RemoteThemeDescriptor,
    assets: &HashMap<String, Vec<u8>>,
) -> Result<ThemeContent, ThemeErrorCode> {
    let mut artwork = BTreeMap::new();
    for (slot, metadata) in &descriptor.artwork {
        let bytes = assets
            .get(&metadata.sha256)
            .ok_or(ThemeErrorCode::AssetsUnavailable)?;
        artwork.insert(
            *slot,
            format!("data:image/png;base64,{}", STANDARD.encode(bytes)),
        );
    }
    Ok(ThemeContent {
        id: descriptor.id.clone(),
        name: descriptor.name.clone(),
        description: descriptor.description.clone(),
        start_month_day: descriptor.start_month_day.clone(),
        end_month_day: descriptor.end_month_day.clone(),
        revision: descriptor.revision,
        palettes: descriptor.palettes.clone(),
        artwork,
    })
}

fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn next_local_midnight_with_offset(
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<OffsetDateTime> {
    let next_date = now.date().next_day()?;
    Some(next_date.with_time(Time::MIDNIGHT).assume_offset(offset))
}

fn duration_until_local_midnight() -> Duration {
    let now = local_now();
    let Some(next_date) = now.date().next_day() else {
        return CLOCK_POLL_INTERVAL;
    };
    let local_wall_time = next_date.with_time(Time::MIDNIGHT);
    let offset =
        UtcOffset::local_offset_at(local_wall_time.assume_utc()).unwrap_or_else(|_| now.offset());
    let Some(next_midnight) = next_local_midnight_with_offset(now, offset) else {
        return CLOCK_POLL_INTERVAL;
    };
    let nanos = (next_midnight - now).whole_nanoseconds();
    if nanos <= 0 {
        return CLOCK_POLL_INTERVAL;
    }
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

fn content_range(content: &ThemeContent) -> Result<(AnnualDate, AnnualDate), ThemeErrorCode> {
    Ok((
        AnnualDate::parse(&content.start_month_day)?,
        AnnualDate::parse(&content.end_month_day)?,
    ))
}

fn is_content_active(content: &ThemeContent, date: Date) -> bool {
    let Ok((start, end)) = content_range(content) else {
        return false;
    };
    is_in_annual_range(date, start, end)
}

fn upcoming_start(content: &ThemeContent, today: Date) -> Option<Date> {
    let start = AnnualDate::parse(&content.start_month_day).ok()?;
    for year_delta in 0..=8 {
        let year = today.year().checked_add(year_delta)?;
        if let Some(candidate) = annual_occurrence_date(start, year) {
            if candidate > today {
                return Some(candidate);
            }
        }
    }
    None
}

fn next_change_at(contents: &[ThemeContent], now: OffsetDateTime) -> Option<String> {
    let mut boundaries = Vec::new();
    let current_date = now.date();
    for content in contents {
        let Ok((start, end)) = content_range(content) else {
            continue;
        };
        for annual_date in [start, end] {
            for year_delta in 0..=8 {
                let Some(year) = current_date.year().checked_add(year_delta) else {
                    continue;
                };
                let Some(date) = annual_occurrence_date(annual_date, year) else {
                    continue;
                };
                let local_wall_time = date.with_time(Time::MIDNIGHT);
                let offset = UtcOffset::local_offset_at(local_wall_time.assume_utc())
                    .unwrap_or_else(|_| now.offset());
                let local_midnight = local_wall_time.assume_offset(offset);
                if local_midnight > now {
                    boundaries.push(local_midnight);
                }
            }
        }
    }
    boundaries
        .into_iter()
        .min()
        .and_then(|boundary| boundary.format(&Rfc3339).ok())
}

fn theme_cache_root(root: &Path) -> PathBuf {
    root.join(THEME_ROOT_DIR)
}

fn valid_snapshot_name(name: &str) -> bool {
    name.len() <= 100
        && name.starts_with("rev-")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn try_with_refresh_gate<T>(gate: &Mutex<()>, operation: impl FnOnce() -> T) -> Option<T> {
    let Ok(_guard) = gate.try_lock() else {
        return None;
    };
    Some(operation())
}

fn read_bounded_file(path: &Path, max_bytes: usize) -> Result<Vec<u8>, ThemeErrorCode> {
    let metadata = fs::metadata(path).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    if metadata.len() > max_bytes as u64 {
        return Err(ThemeErrorCode::CacheUnavailable);
    }
    let bytes = fs::read(path).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    if bytes.len() > max_bytes {
        return Err(ThemeErrorCode::CacheUnavailable);
    }
    Ok(bytes)
}

fn read_cache_pointer(root: &Path) -> Result<Option<CachePointer>, ThemeErrorCode> {
    let path = theme_cache_root(root).join("state.json");
    let bytes = match read_bounded_file(&path, 4096) {
        Ok(bytes) => bytes,
        Err(ThemeErrorCode::CacheUnavailable) if !path.exists() => return Ok(None),
        Err(error) => return Err(error),
    };
    let pointer: CachePointer =
        serde_json::from_slice(&bytes).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    if !valid_snapshot_name(&pointer.active_snapshot)
        || pointer.high_water_revision == 0
        || pointer.active_payload_sha256.len() != 64
        || !pointer
            .active_payload_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ThemeErrorCode::CacheUnavailable);
    }
    Ok(Some(pointer))
}

fn load_cached_snapshot(
    root: &Path,
) -> Result<Option<(u64, ThemeCatalogPayload, BTreeMap<String, ThemeContent>)>, ThemeErrorCode> {
    let public_key = pinned_public_key_text()?;
    load_cached_snapshot_with_key(root, &public_key)
}

fn load_cached_snapshot_with_key(
    root: &Path,
    public_key: &str,
) -> Result<Option<(u64, ThemeCatalogPayload, BTreeMap<String, ThemeContent>)>, ThemeErrorCode> {
    let Some(pointer) = recover_cache_pointer(root, public_key)? else {
        return Ok(None);
    };
    load_snapshot_content(root, &pointer, public_key).map(Some)
}

fn prefer_pointer_target_for_tied_snapshot(
    current_best: &CachePointer,
    candidate: &CachePointer,
    preferred_pointer: Option<&CachePointer>,
) -> bool {
    let Some(preferred_pointer) = preferred_pointer else {
        return false;
    };
    current_best.high_water_revision == candidate.high_water_revision
        && current_best.active_payload_sha256 == candidate.active_payload_sha256
        && candidate.high_water_revision == preferred_pointer.high_water_revision
        && candidate.active_payload_sha256 == preferred_pointer.active_payload_sha256
        && candidate.active_snapshot == preferred_pointer.active_snapshot
        && current_best.active_snapshot != preferred_pointer.active_snapshot
}

fn load_snapshot_catalog(
    root: &Path,
    snapshot_name: &str,
    public_key: &str,
) -> Result<(CachePointer, ThemeCatalogPayload), ThemeErrorCode> {
    if !valid_snapshot_name(snapshot_name) {
        return Err(ThemeErrorCode::CacheUnavailable);
    }
    let snapshot_dir = theme_cache_root(root).join("snapshots").join(snapshot_name);
    let envelope_bytes = read_bounded_file(&snapshot_dir.join("catalog.json"), MAX_ENVELOPE_BYTES)?;
    let (payload_bytes, catalog) = verify_signed_catalog(&envelope_bytes, public_key)?;
    let payload_hash = sha256_hex(&payload_bytes);
    let pointer = CachePointer {
        high_water_revision: catalog.revision,
        active_snapshot: snapshot_name.to_owned(),
        active_payload_sha256: payload_hash,
    };
    Ok((pointer, catalog))
}

fn quarantine_cache_pointer(root: &Path) -> Result<(), ThemeErrorCode> {
    let cache_root = theme_cache_root(root);
    let state_path = cache_root.join("state.json");
    if state_path.exists() {
        let quarantine = cache_root.join(format!(
            "state.quarantine-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::rename(&state_path, quarantine).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    }
    let mut quarantined = fs::read_dir(&cache_root)
        .map_err(|_| ThemeErrorCode::CacheUnavailable)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("state.quarantine-")
        })
        .collect::<Vec<_>>();
    quarantined.sort_by_key(|entry| entry.path());
    let remove_count = quarantined.len().saturating_sub(3);
    for old in quarantined.into_iter().take(remove_count) {
        let _ = fs::remove_file(old.path());
    }
    Ok(())
}

fn recover_cache_pointer(
    root: &Path,
    public_key: &str,
) -> Result<Option<CachePointer>, ThemeErrorCode> {
    let state_path = theme_cache_root(root).join("state.json");
    let pointer_file_exists = state_path.exists();
    let has_quarantined_pointer = theme_cache_root(root)
        .read_dir()
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("state.quarantine-")
        });
    let snapshots_root = theme_cache_root(root).join("snapshots");
    let mut entries = match fs::read_dir(&snapshots_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if pointer_file_exists || has_quarantined_pointer {
                if pointer_file_exists {
                    quarantine_cache_pointer(root)?;
                }
                warn!(
                    reason = "cache-reset-no-authenticated-revision",
                    "seasonal theme cache reset; no authenticated revision survived"
                );
                return Ok(None);
            }
            return Ok(None);
        }
        Err(_) => return Err(ThemeErrorCode::CacheUnavailable),
    };
    let mut candidate_names = Vec::new();
    for entry in entries.by_ref().take(MAX_SNAPSHOT_SCAN + 1) {
        let entry = entry.map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        let file_type = entry
            .file_type()
            .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if valid_snapshot_name(&name) {
                candidate_names.push(name);
            }
        }
    }
    if entries.next().is_some() {
        return Err(ThemeErrorCode::CacheUnavailable);
    }

    candidate_names.sort_unstable();
    let preferred_pointer = read_cache_pointer(root).ok().flatten();
    let mut best: Option<(CachePointer, ThemeCatalogPayload)> = None;
    for name in candidate_names {
        let Ok((pointer, catalog)) = load_snapshot_catalog(root, &name, public_key) else {
            continue;
        };
        match &best {
            Some((current, _)) if current.high_water_revision > pointer.high_water_revision => {}
            Some((current, _)) if current.high_water_revision == pointer.high_water_revision => {
                if current.active_payload_sha256 != pointer.active_payload_sha256 {
                    if pointer_file_exists {
                        quarantine_cache_pointer(root)?;
                    }
                    return Err(ThemeErrorCode::CacheUnavailable);
                }
                if prefer_pointer_target_for_tied_snapshot(
                    current,
                    &pointer,
                    preferred_pointer.as_ref(),
                ) {
                    best = Some((pointer, catalog));
                }
            }
            _ => best = Some((pointer, catalog)),
        }
    }

    if let Some((pointer, _)) = best {
        let current_pointer = read_cache_pointer(root).ok().flatten();
        if current_pointer.as_ref().is_some_and(|current| {
            current.high_water_revision == pointer.high_water_revision
                && current.active_snapshot == pointer.active_snapshot
                && current.active_payload_sha256 == pointer.active_payload_sha256
        }) {
            return Ok(Some(pointer));
        }
        if pointer_file_exists && current_pointer.is_none() {
            quarantine_cache_pointer(root)?;
        } else {
            fs::create_dir_all(theme_cache_root(root))
                .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        }
        let bytes = serde_json::to_vec(&pointer).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        atomic_write(&state_path, &bytes)?;
        return Ok(Some(pointer));
    }

    if pointer_file_exists {
        quarantine_cache_pointer(root)?;
    }
    if pointer_file_exists || has_quarantined_pointer {
        warn!(
            reason = "cache-reset-no-authenticated-revision",
            "seasonal theme cache reset; no authenticated revision survived"
        );
    }
    Ok(None)
}

fn load_snapshot_content(
    root: &Path,
    pointer: &CachePointer,
    public_key: &str,
) -> Result<(u64, ThemeCatalogPayload, BTreeMap<String, ThemeContent>), ThemeErrorCode> {
    let snapshot_dir = theme_cache_root(root)
        .join("snapshots")
        .join(&pointer.active_snapshot);
    let envelope_path = snapshot_dir.join("catalog.json");
    let envelope_bytes = read_bounded_file(&envelope_path, MAX_ENVELOPE_BYTES)?;
    let (payload_bytes, catalog) = verify_signed_catalog(&envelope_bytes, public_key)?;
    let payload_hash = sha256_hex(&payload_bytes);
    if catalog.revision != pointer.high_water_revision
        || payload_hash != pointer.active_payload_sha256
    {
        return Err(ThemeErrorCode::CacheUnavailable);
    }

    let manifest_bytes = read_bounded_file(&snapshot_dir.join("manifest.json"), 4096)?;
    let manifest: SnapshotManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    if manifest.selected_theme_ids.len() > MAX_CACHED_REMOTE_THEMES {
        return Err(ThemeErrorCode::CacheUnavailable);
    }
    let mut selected = BTreeSet::new();
    for id in &manifest.selected_theme_ids {
        if !validate_theme_id(id)
            || !selected.insert(id.as_str())
            || !catalog.themes.iter().any(|theme| theme.id == *id)
        {
            return Err(ThemeErrorCode::CacheUnavailable);
        }
    }

    let mut assets = HashMap::new();
    let mut total_size = 0_u64;
    for theme in catalog
        .themes
        .iter()
        .filter(|theme| selected.contains(theme.id.as_str()))
    {
        for metadata in theme.artwork.values() {
            if assets.contains_key(&metadata.sha256) {
                continue;
            }
            let asset_path = snapshot_dir
                .join("assets")
                .join(format!("{}.png", metadata.sha256));
            let bytes = read_bounded_file(&asset_path, MAX_IMAGE_BYTES)?;
            validate_artwork_bytes(metadata, &bytes)
                .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
            total_size = total_size
                .checked_add(bytes.len() as u64)
                .ok_or(ThemeErrorCode::CacheUnavailable)?;
            if total_size > MAX_CACHE_IMAGE_BYTES as u64 {
                return Err(ThemeErrorCode::CacheUnavailable);
            }
            assets.insert(metadata.sha256.clone(), bytes);
        }
    }
    let mut content = BTreeMap::new();
    for theme in catalog
        .themes
        .iter()
        .filter(|theme| selected.contains(theme.id.as_str()))
    {
        let item =
            content_from_remote(theme, &assets).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        content.insert(item.id.clone(), item);
    }
    Ok((catalog.revision, catalog, content))
}

fn bounded_response_body(
    mut response: Response,
    max_bytes: usize,
) -> Result<Vec<u8>, ThemeErrorCode> {
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
    if bytes.len() > max_bytes {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }
    Ok(bytes)
}

fn catalog_client() -> Result<Client, ThemeErrorCode> {
    Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .build()
        .map_err(|_| ThemeErrorCode::CatalogUnavailable)
}

fn asset_redirect_url_is_allowed(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("release-assets.githubusercontent.com") | Some("objects.githubusercontent.com")
    ) && url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && !url.path().contains('\\')
}

fn asset_redirect_is_allowed(url: &Url, previous_redirects: usize) -> bool {
    previous_redirects < 4 && asset_redirect_url_is_allowed(url)
}

fn asset_client() -> Result<Client, ThemeErrorCode> {
    Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(30))
        .redirect(Policy::custom(|attempt| {
            if !asset_redirect_is_allowed(attempt.url(), attempt.previous().len()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| ThemeErrorCode::AssetsUnavailable)
}

fn fetch_catalog() -> Result<(Vec<u8>, Vec<u8>, ThemeCatalogPayload), ThemeErrorCode> {
    let client = catalog_client()?;
    let response = client
        .get(CATALOG_URL)
        .send()
        .map_err(|_| ThemeErrorCode::CatalogUnavailable)?;
    let envelope_bytes = bounded_response_body(response, MAX_ENVELOPE_BYTES)
        .map_err(|_| ThemeErrorCode::CatalogUnavailable)?;
    let public_key = pinned_public_key_text()?;
    let (payload_bytes, catalog) = verify_signed_catalog(&envelope_bytes, &public_key)?;
    Ok((envelope_bytes, payload_bytes, catalog))
}

fn date_is_active(theme: &RemoteThemeDescriptor, date: Date) -> bool {
    let (Ok(start), Ok(end)) = (
        AnnualDate::parse(&theme.start_month_day),
        AnnualDate::parse(&theme.end_month_day),
    ) else {
        return false;
    };
    is_in_annual_range(date, start, end)
}

fn relevant_theme_ids(catalog: &ThemeCatalogPayload, date: Date) -> Vec<String> {
    let active = catalog
        .themes
        .iter()
        .find(|theme| date_is_active(theme, date));
    let mut ids = active
        .map(|theme| vec![theme.id.clone()])
        .unwrap_or_default();
    let upcoming = catalog
        .themes
        .iter()
        .filter_map(|theme| {
            let content = ThemeContent {
                id: theme.id.clone(),
                name: theme.name.clone(),
                description: theme.description.clone(),
                start_month_day: theme.start_month_day.clone(),
                end_month_day: theme.end_month_day.clone(),
                revision: theme.revision,
                palettes: theme.palettes.clone(),
                artwork: BTreeMap::new(),
            };
            upcoming_start(&content, date).map(|start| (start, theme))
        })
        .min_by_key(|(start, _)| *start)
        .map(|(_, theme)| theme);
    if let Some(theme) = upcoming {
        if !ids.iter().any(|id| id == &theme.id) {
            ids.push(theme.id.clone());
        }
    }
    ids.truncate(MAX_CACHED_REMOTE_THEMES);
    ids
}

fn download_artwork(
    catalog: &ThemeCatalogPayload,
    selected_ids: &[String],
) -> Result<HashMap<String, Vec<u8>>, ThemeErrorCode> {
    let selected = catalog
        .themes
        .iter()
        .filter(|theme| selected_ids.iter().any(|id| id == &theme.id))
        .collect::<Vec<_>>();
    let mut metadata_by_hash = BTreeMap::<String, &RemoteArtwork>::new();
    for theme in &selected {
        for artwork in theme.artwork.values() {
            metadata_by_hash
                .entry(artwork.sha256.clone())
                .or_insert(artwork);
        }
    }
    let total_size = metadata_by_hash
        .values()
        .try_fold(0_u64, |sum, artwork| sum.checked_add(artwork.size_bytes));
    if total_size.is_none_or(|total| total > MAX_CACHE_IMAGE_BYTES as u64) {
        return Err(ThemeErrorCode::AssetsUnavailable);
    }

    let client = asset_client()?;
    let mut assets = HashMap::new();
    for (hash, artwork) in metadata_by_hash {
        let response = client
            .get(&artwork.url)
            .send()
            .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
        if response.url().host_str() != Some("github.com")
            && !asset_redirect_url_is_allowed(response.url())
        {
            return Err(ThemeErrorCode::AssetsUnavailable);
        }
        let bytes = bounded_response_body(response, MAX_IMAGE_BYTES)?;
        validate_artwork_bytes(artwork, &bytes)?;
        assets.insert(hash, bytes);
    }
    Ok(assets)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ThemeErrorCode> {
    let parent = path.parent().ok_or(ThemeErrorCode::CacheUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    let temp_name = format!(
        "{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or(ThemeErrorCode::CacheUnavailable)?,
        uuid::Uuid::new_v4().simple()
    );
    let temp_path = parent.join(temp_name);
    {
        let mut file =
            fs::File::create(&temp_path).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        file.write_all(bytes)
            .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        file.sync_all()
            .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    }
    replace_file(&temp_path, path)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> Result<(), ThemeErrorCode> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from_wide = from
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let to_wide = to
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        let _ = fs::remove_file(from);
        Err(ThemeErrorCode::CacheUnavailable)
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> Result<(), ThemeErrorCode> {
    fs::rename(from, to).map_err(|_| ThemeErrorCode::CacheUnavailable)
}

fn commit_snapshot(
    root: &Path,
    envelope_bytes: &[u8],
    payload_bytes: &[u8],
    catalog: &ThemeCatalogPayload,
    selected_theme_ids: &[String],
    assets: &HashMap<String, Vec<u8>>,
    previous_pointer: Option<&CachePointer>,
) -> Result<(), ThemeErrorCode> {
    let payload_hash = sha256_hex(payload_bytes);
    let previous_high_water = previous_pointer
        .map(|pointer| pointer.high_water_revision)
        .unwrap_or(0);
    if catalog.revision < previous_high_water {
        return Err(ThemeErrorCode::RollbackDetected);
    }
    if catalog.revision == previous_high_water
        && previous_pointer.is_some_and(|pointer| pointer.active_payload_sha256 != payload_hash)
    {
        return Err(ThemeErrorCode::RollbackDetected);
    }

    let cache_root = theme_cache_root(root);
    let snapshots_root = cache_root.join("snapshots");
    fs::create_dir_all(&snapshots_root).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    let staging_name = format!(".staging-{}", uuid::Uuid::new_v4().simple());
    let staging = snapshots_root.join(&staging_name);
    fs::create_dir_all(staging.join("assets")).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    let write_result = (|| {
        fs::write(staging.join("catalog.json"), envelope_bytes)
            .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        let manifest = SnapshotManifest {
            selected_theme_ids: selected_theme_ids.to_vec(),
        };
        let manifest_bytes =
            serde_json::to_vec(&manifest).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        fs::write(staging.join("manifest.json"), manifest_bytes)
            .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        for (hash, bytes) in assets {
            fs::write(staging.join("assets").join(format!("{hash}.png")), bytes)
                .map_err(|_| ThemeErrorCode::CacheUnavailable)?;
        }
        Ok::<(), ThemeErrorCode>(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    let snapshot_name = format!(
        "rev-{}-{}-{}",
        catalog.revision,
        &payload_hash[..16],
        uuid::Uuid::new_v4().simple()
    );
    let snapshot = snapshots_root.join(&snapshot_name);
    fs::rename(&staging, &snapshot).map_err(|_| {
        let _ = fs::remove_dir_all(&staging);
        ThemeErrorCode::CacheUnavailable
    })?;
    let pointer = CachePointer {
        high_water_revision: catalog.revision.max(previous_high_water),
        active_snapshot: snapshot_name,
        active_payload_sha256: payload_hash,
    };
    let pointer_bytes =
        serde_json::to_vec(&pointer).map_err(|_| ThemeErrorCode::CacheUnavailable)?;
    if let Err(error) = atomic_write(&cache_root.join("state.json"), &pointer_bytes) {
        let _ = fs::remove_dir_all(&snapshot);
        return Err(error);
    }
    if let Ok(entries) = fs::read_dir(&snapshots_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path != snapshot && path.is_dir() {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
    Ok(())
}

impl SeasonalThemeManager {
    pub fn new(app: &AppHandle, settings: &Settings) -> Self {
        #[cfg(windows)]
        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Lyra"));
        #[cfg(not(windows))]
        let root: Option<PathBuf> = None;
        let root = root.unwrap_or_else(|| {
            app.path()
                .app_local_data_dir()
                .unwrap_or_else(|_| PathBuf::from("app-data"))
                .join("Lyra")
        });

        Self {
            inner: Arc::new(ThemeManagerInner {
                app: app.clone(),
                root,
                enabled: AtomicBool::new(settings.seasonal_themes_enabled),
                motion_enabled: AtomicBool::new(settings.seasonal_motion_enabled),
                state: Mutex::new(ThemeRuntimeState::default()),
                emission_gate: Mutex::new(()),
                last_emitted: Mutex::new(None),
                refresh_gate: Mutex::new(()),
                refresh_timer: Mutex::new(RefreshTimer::default()),
                started: AtomicBool::new(false),
            }),
        }
    }

    pub fn start(&self) {
        if self.inner.started.swap(true, Ordering::Relaxed) {
            return;
        }
        let manager = self.clone();
        let _ = thread::Builder::new()
            .name("lyra-seasonal-themes".to_owned())
            .spawn(move || {
                manager.load_cached_state();
                manager.emit_if_changed();
                if manager.is_enabled() {
                    manager.refresh_now();
                }

                let mut previous = ClockObservation::now();
                loop {
                    thread::sleep(CLOCK_POLL_INTERVAL.min(duration_until_local_midnight()));
                    let current = ClockObservation::now();
                    if current.calendar_changed_from(previous) {
                        manager.emit_if_changed();
                    }
                    if manager.is_enabled() && manager.refresh_is_due() {
                        manager.refresh_now();
                    }
                    previous = current;
                }
            });
    }

    pub fn update_preferences(&self, enabled: bool, motion_enabled: bool) {
        let was_enabled = if let Ok(_emission_guard) = self.inner.emission_gate.lock() {
            let was_enabled = self.inner.enabled.swap(enabled, Ordering::Relaxed);
            self.inner
                .motion_enabled
                .store(motion_enabled, Ordering::Relaxed);
            self.emit_if_changed_locked();
            was_enabled
        } else {
            let was_enabled = self.inner.enabled.swap(enabled, Ordering::Relaxed);
            self.inner
                .motion_enabled
                .store(motion_enabled, Ordering::Relaxed);
            was_enabled
        };
        if enabled && !was_enabled {
            let manager = self.clone();
            let _ = thread::Builder::new()
                .name("lyra-seasonal-theme-refresh".to_owned())
                .spawn(move || {
                    manager.refresh_now();
                });
        }
    }

    pub fn on_resume(&self) {
        self.emit_if_changed();
        if self.is_enabled() && self.refresh_is_due() {
            let manager = self.clone();
            let _ = thread::Builder::new()
                .name("lyra-seasonal-theme-resume".to_owned())
                .spawn(move || {
                    manager.refresh_now();
                });
        }
    }

    pub fn status(&self) -> ThemeStatus {
        self.build_status()
    }

    pub fn content(&self, theme_id: Option<&str>) -> Result<Option<ThemeContent>, &'static str> {
        if let Some(id) = theme_id {
            if !validate_theme_id(id) {
                return Err("invalid-theme-id");
            }
            if let Ok(state) = self.inner.state.lock() {
                if let Some(content) = state.remote_content.get(id) {
                    return Ok(Some(content.clone()));
                }
            }
            return if id == "october" {
                bundled_theme().map(Some).map_err(|_| "theme-unavailable")
            } else {
                Ok(None)
            };
        }
        Ok(self.build_status().content)
    }

    pub fn refresh_now(&self) -> ThemeStatus {
        if !self.is_enabled() {
            return self.build_status();
        }
        let Some(()) = try_with_refresh_gate(&self.inner.refresh_gate, || {
            let refreshed = self.refresh_catalog_and_assets();
            if let Ok(mut timer) = self.inner.refresh_timer.lock() {
                timer.last_attempt = Some(Instant::now());
                timer.failed = refreshed.is_err();
            }
            match refreshed {
                Ok((catalog, content)) => {
                    if let Ok(mut state) = self.inner.state.lock() {
                        state.revision = Some(catalog.revision);
                        state.catalog = Some(catalog);
                        state.remote_content = content;
                        state.cache_source = false;
                        state.refreshed_at = Some(OffsetDateTime::now_utc());
                        state.last_error = None;
                    }
                    info!("seasonal theme catalog refreshed");
                }
                Err(error) => {
                    if let Ok(mut state) = self.inner.state.lock() {
                        state.last_error = Some(error);
                    }
                    warn!(error_code = error.as_str(), "seasonal theme refresh failed");
                }
            }
        }) else {
            return self.build_status();
        };
        self.emit_if_changed();
        self.build_status()
    }

    fn is_enabled(&self) -> bool {
        self.inner.enabled.load(Ordering::Relaxed)
    }

    fn refresh_is_due(&self) -> bool {
        let Ok(timer) = self.inner.refresh_timer.lock() else {
            return false;
        };
        let interval = if timer.failed {
            FAILURE_RETRY_INTERVAL
        } else {
            REFRESH_INTERVAL
        };
        timer
            .last_attempt
            .is_none_or(|last_attempt| last_attempt.elapsed() >= interval)
    }

    fn load_cached_state(&self) {
        let Some(cached) = try_with_refresh_gate(&self.inner.refresh_gate, || {
            load_cached_snapshot(&self.inner.root)
        }) else {
            return;
        };
        match cached {
            Ok(Some((revision, catalog, remote_content))) => {
                if let Ok(mut state) = self.inner.state.lock() {
                    state.revision = Some(revision);
                    state.catalog = Some(catalog);
                    state.remote_content = remote_content;
                    state.cache_source = true;
                    state.last_error = None;
                }
                info!("seasonal theme cache verified");
            }
            Ok(None) => {}
            Err(error) => {
                if let Ok(mut state) = self.inner.state.lock() {
                    state.last_error = Some(error);
                }
                warn!(error_code = error.as_str(), "seasonal theme cache rejected");
            }
        }
    }

    fn refresh_catalog_and_assets(
        &self,
    ) -> Result<(ThemeCatalogPayload, BTreeMap<String, ThemeContent>), ThemeErrorCode> {
        let public_key = pinned_public_key_text()?;
        let previous_pointer = recover_cache_pointer(&self.inner.root, &public_key)?;
        let (envelope_bytes, payload_bytes, catalog) = fetch_catalog()?;
        let payload_hash = sha256_hex(&payload_bytes);
        let state_revision = self
            .inner
            .state
            .lock()
            .ok()
            .and_then(|state| state.revision)
            .unwrap_or(0);
        let high_water = previous_pointer
            .as_ref()
            .map(|pointer| pointer.high_water_revision)
            .unwrap_or(state_revision);
        if catalog.revision < high_water
            || (catalog.revision == high_water
                && previous_pointer
                    .as_ref()
                    .is_some_and(|pointer| pointer.active_payload_sha256 != payload_hash))
        {
            return Err(ThemeErrorCode::RollbackDetected);
        }

        let selected_ids = relevant_theme_ids(&catalog, local_now().date());
        let assets = download_artwork(&catalog, &selected_ids)?;
        commit_snapshot(
            &self.inner.root,
            &envelope_bytes,
            &payload_bytes,
            &catalog,
            &selected_ids,
            &assets,
            previous_pointer.as_ref(),
        )?;
        let mut remote_content = BTreeMap::new();
        for theme in catalog
            .themes
            .iter()
            .filter(|theme| selected_ids.iter().any(|id| id == &theme.id))
        {
            let content = content_from_remote(theme, &assets)?;
            remote_content.insert(content.id.clone(), content);
        }
        Ok((catalog, remote_content))
    }

    fn build_status(&self) -> ThemeStatus {
        let now = local_now();
        let state_guard = self.inner.state.lock().ok();
        let state = state_guard.as_deref();
        let enabled = self.inner.enabled.load(Ordering::Relaxed);
        let motion_enabled = self.inner.motion_enabled.load(Ordering::Relaxed);
        let remote_content = state
            .map(|state| state.remote_content.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut all_content = remote_content.clone();
        let bundle = bundled_theme().ok();
        if let Some(bundle) = &bundle {
            all_content.push(bundle.clone());
        }

        let active_remote = if enabled {
            remote_content
                .iter()
                .find(|theme| is_content_active(theme, now.date()))
                .cloned()
        } else {
            None
        };
        let active_bundle = if enabled {
            bundle
                .as_ref()
                .filter(|theme| is_content_active(theme, now.date()))
                .cloned()
        } else {
            None
        };
        let upcoming = if enabled {
            all_content
                .iter()
                .filter_map(|theme| upcoming_start(theme, now.date()).map(|date| (date, theme)))
                .min_by_key(|(date, _)| *date)
                .is_some()
        } else {
            false
        };

        let (status_state, content, source) = if !enabled {
            (ThemeState::Disabled, None, None)
        } else if let Some(content) = active_remote {
            let source = if state.is_some_and(|state| state.cache_source) {
                ThemeSource::Cache
            } else {
                ThemeSource::Remote
            };
            (ThemeState::Active, Some(content), Some(source))
        } else if let Some(content) = active_bundle {
            (
                ThemeState::Fallback,
                Some(content),
                Some(ThemeSource::Bundled),
            )
        } else if upcoming {
            (ThemeState::Upcoming, None, None)
        } else {
            (ThemeState::Unavailable, None, None)
        };

        let mut available = BTreeMap::new();
        for theme in &remote_content {
            available.insert(
                theme.id.clone(),
                AvailableTheme {
                    id: theme.id.clone(),
                    name: theme.name.clone(),
                    revision: theme.revision,
                    start_month_day: theme.start_month_day.clone(),
                    end_month_day: theme.end_month_day.clone(),
                    source: if state.is_some_and(|state| state.cache_source) {
                        ThemeSource::Cache
                    } else {
                        ThemeSource::Remote
                    },
                },
            );
        }
        if let Some(bundle) = &bundle {
            available
                .entry(bundle.id.clone())
                .or_insert_with(|| AvailableTheme {
                    id: bundle.id.clone(),
                    name: bundle.name.clone(),
                    revision: bundle.revision,
                    start_month_day: bundle.start_month_day.clone(),
                    end_month_day: bundle.end_month_day.clone(),
                    source: ThemeSource::Bundled,
                });
        }

        let next_change_at = enabled.then(|| next_change_at(&all_content, now)).flatten();
        let refreshed_at = state
            .and_then(|state| state.refreshed_at)
            .and_then(|time| time.format(&Rfc3339).ok());
        let error = state.and_then(|state| state.last_error.map(|code| code.as_str().to_owned()));

        ThemeStatus {
            enabled,
            motion_enabled,
            state: status_state,
            theme_id: content.as_ref().map(|theme| theme.id.clone()),
            catalog_revision: state.and_then(|state| state.revision),
            next_change_at,
            refreshed_at,
            error,
            source,
            available_themes: available.into_values().collect(),
            content,
        }
    }

    fn emit_if_changed(&self) {
        let Ok(_emission_guard) = self.inner.emission_gate.lock() else {
            return;
        };
        self.emit_if_changed_locked();
    }

    fn emit_if_changed_locked(&self) {
        let status = self.build_status();
        let should_emit = if let Ok(mut previous) = self.inner.last_emitted.lock() {
            if previous.as_ref() == Some(&status) {
                false
            } else {
                *previous = Some(status.clone());
                true
            }
        } else {
            true
        };
        if should_emit {
            let _ = self.inner.app.emit(THEME_CHANGED_EVENT, status);
        }
    }
}

impl ClockObservation {
    fn now() -> Self {
        let local = local_now();
        Self {
            utc_timestamp: OffsetDateTime::now_utc().unix_timestamp(),
            local_date: local.date(),
            offset_seconds: local.offset().whole_seconds(),
            monotonic: Instant::now(),
        }
    }

    fn calendar_changed_from(self, previous: Self) -> bool {
        let wall_seconds = self.utc_timestamp.saturating_sub(previous.utc_timestamp);
        let elapsed_seconds = self
            .monotonic
            .saturating_duration_since(previous.monotonic)
            .as_secs() as i64;
        self.local_date != previous.local_date
            || self.offset_seconds != previous.offset_seconds
            || wall_seconds.saturating_sub(elapsed_seconds).abs() > 90
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tied_snapshot_recovery_prefers_the_valid_pointer_target() {
        let stale = CachePointer {
            high_water_revision: 7,
            active_snapshot: "rev-7-payload-stale".to_owned(),
            active_payload_sha256: "payload-hash".to_owned(),
        };
        let repaired = CachePointer {
            high_water_revision: 7,
            active_snapshot: "rev-7-payload-repaired".to_owned(),
            active_payload_sha256: "payload-hash".to_owned(),
        };

        assert!(prefer_pointer_target_for_tied_snapshot(
            &stale,
            &repaired,
            Some(&repaired)
        ));
        assert!(!prefer_pointer_target_for_tied_snapshot(
            &repaired,
            &stale,
            Some(&repaired)
        ));
        assert!(!prefer_pointer_target_for_tied_snapshot(
            &stale, &repaired, None
        ));
    }

    struct TemporaryCache(PathBuf);

    impl TemporaryCache {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "lyra-seasonal-themes-test-{}",
                uuid::Uuid::new_v4().simple()
            )))
        }
    }

    impl Drop for TemporaryCache {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct SignedFixture {
        envelope: Vec<u8>,
        public_key: String,
        asset_dir: Option<PathBuf>,
    }

    fn signed_fixture_from_environment() -> Option<SignedFixture> {
        let envelope = std::env::var_os("LYRA_THEMES_FIXTURE_ENVELOPE");
        let public_key = std::env::var_os("LYRA_THEMES_FIXTURE_PUBLIC_KEY");
        match (envelope, public_key) {
            (None, None) => None,
            (Some(envelope), Some(public_key)) => Some(SignedFixture {
                envelope: fs::read(envelope).expect("fixture envelope should be readable"),
                public_key: fs::read_to_string(public_key)
                    .expect("fixture public key should be readable"),
                asset_dir: std::env::var_os("LYRA_THEMES_FIXTURE_ASSET_DIR").map(PathBuf::from),
            }),
            _ => panic!("both signed fixture envelope and public key paths are required"),
        }
    }

    fn fixture_assets(
        catalog: &ThemeCatalogPayload,
        asset_dir: &Path,
        selected_theme_ids: Option<&[String]>,
    ) -> Result<HashMap<String, Vec<u8>>, ThemeErrorCode> {
        let mut assets = HashMap::new();
        let mut total_size = 0_u64;
        for theme in catalog.themes.iter().filter(|theme| {
            selected_theme_ids.is_none_or(|ids| ids.iter().any(|id| id == &theme.id))
        }) {
            for metadata in theme.artwork.values() {
                if assets.contains_key(&metadata.sha256) {
                    continue;
                }
                let bytes = fs::read(asset_dir.join(format!("{}.png", metadata.sha256)))
                    .map_err(|_| ThemeErrorCode::AssetsUnavailable)?;
                validate_artwork_bytes(metadata, &bytes)?;
                total_size = total_size
                    .checked_add(bytes.len() as u64)
                    .ok_or(ThemeErrorCode::AssetsUnavailable)?;
                if selected_theme_ids.is_some() && total_size > MAX_CACHE_IMAGE_BYTES as u64 {
                    return Err(ThemeErrorCode::AssetsUnavailable);
                }
                assets.insert(metadata.sha256.clone(), bytes);
            }
        }
        Ok(assets)
    }

    fn fixture_catalog(fixture: &SignedFixture) -> (Vec<u8>, ThemeCatalogPayload) {
        let (payload, catalog) = verify_signed_catalog(&fixture.envelope, &fixture.public_key)
            .expect("signed fixture should verify");
        (payload, catalog)
    }

    fn bundled_png_for_slot(slot: ArtworkSlot) -> &'static [u8] {
        match slot {
            ArtworkSlot::Ambience => BUNDLED_LEAF_PNG,
            _ => BUNDLED_PUMPKIN_PNG,
        }
    }

    fn sample_remote_theme(id: &str, start: &str, end: &str) -> RemoteThemeDescriptor {
        let bundled = load_bundled_theme().expect("bundled descriptor should validate");
        let artwork = ARTWORK_SLOTS
            .into_iter()
            .map(|slot| {
                let bytes = bundled_png_for_slot(slot);
                (
                    slot,
                    RemoteArtwork {
                        url: format!(
                            "https://github.com/OrionAISystems/Lyra-Releases/releases/download/themes-{id}-v1/{}.png",
                            if slot == ArtworkSlot::Ambience {
                                "leaf"
                            } else {
                                "pumpkin"
                            }
                        ),
                        sha256: sha256_hex(bytes),
                        size_bytes: bytes.len() as u64,
                    },
                )
            })
            .collect();
        RemoteThemeDescriptor {
            id: id.to_owned(),
            name: "Test Autumn".to_owned(),
            description: "A test palette".to_owned(),
            start_month_day: start.to_owned(),
            end_month_day: end.to_owned(),
            revision: 1,
            palettes: bundled.palettes,
            artwork,
        }
    }

    fn sample_catalog(themes: Vec<RemoteThemeDescriptor>) -> ThemeCatalogPayload {
        ThemeCatalogPayload {
            discriminator: CATALOG_DISCRIMINATOR.to_owned(),
            schema_version: CATALOG_SCHEMA_VERSION,
            engine_version: ENGINE_VERSION,
            revision: 1,
            themes,
        }
    }

    #[test]
    fn bundled_theme_is_valid_and_contains_all_data_url_slots() {
        let theme = bundled_theme().expect("bundled October theme should validate");
        assert_eq!(theme.id, "october");
        assert_eq!(theme.start_month_day, "10-01");
        assert_eq!(theme.end_month_day, "11-01");
        validate_artwork_slots(&theme.artwork).expect("all fixed artwork slots should exist");
        assert!(theme
            .artwork
            .values()
            .all(|url| url.starts_with("data:image/png;base64,")));
        assert!(theme.artwork.values().all(|url| !url.contains("<svg")));
    }

    #[test]
    fn catalog_schema_palette_urls_and_annual_ranges_are_strict() {
        let october = sample_remote_theme("october", "10-01", "11-01");
        let bytes = serde_json::to_vec(&sample_catalog(vec![october.clone()])).unwrap();
        assert!(validate_catalog_payload(&bytes).is_ok());

        let mut unknown_field: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        unknown_field["unapproved"] = serde_json::Value::Bool(true);
        assert!(matches!(
            validate_catalog_payload(&serde_json::to_vec(&unknown_field).unwrap()),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let overlapping = sample_remote_theme("autumn-two", "10-15", "11-15");
        assert!(matches!(
            validate_catalog_payload(
                &serde_json::to_vec(&sample_catalog(vec![october.clone(), overlapping])).unwrap()
            ),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut wrong_engine = sample_catalog(vec![october.clone()]);
        wrong_engine.engine_version += 1;
        assert!(matches!(
            validate_catalog_payload(&serde_json::to_vec(&wrong_engine).unwrap()),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut invalid_url = october;
        invalid_url.artwork.get_mut(&ArtworkSlot::Idle).unwrap().url =
            "https://example.com/pumpkin.png".to_owned();
        assert!(matches!(
            validate_catalog_payload(
                &serde_json::to_vec(&sample_catalog(vec![invalid_url])).unwrap()
            ),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut wrong_tag = sample_remote_theme("october", "10-01", "11-01");
        wrong_tag.artwork.get_mut(&ArtworkSlot::Idle).unwrap().url =
            "https://github.com/OrionAISystems/Lyra-Releases/releases/download/themes-other-v1/pumpkin.png".to_owned();
        assert!(matches!(
            validate_catalog_payload(
                &serde_json::to_vec(&sample_catalog(vec![wrong_tag])).unwrap()
            ),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut wrong_schema =
            sample_catalog(vec![sample_remote_theme("october", "10-01", "11-01")]);
        wrong_schema.schema_version += 1;
        assert!(matches!(
            validate_catalog_payload(&serde_json::to_vec(&wrong_schema).unwrap()),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut oversized_image = sample_remote_theme("october", "10-01", "11-01");
        oversized_image
            .artwork
            .get_mut(&ArtworkSlot::Idle)
            .unwrap()
            .size_bytes = MAX_IMAGE_BYTES as u64 + 1;
        assert!(matches!(
            validate_catalog_payload(
                &serde_json::to_vec(&sample_catalog(vec![oversized_image])).unwrap()
            ),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let mut oversized_pack = sample_remote_theme("test-pack", "10-01", "11-01");
        for (index, slot) in ARTWORK_SLOTS.iter().take(5).enumerate() {
            let metadata = oversized_pack.artwork.get_mut(slot).unwrap();
            metadata.url = format!(
                "https://github.com/OrionAISystems/Lyra-Releases/releases/download/themes-test-pack-v1/{index}.png"
            );
            metadata.sha256 = format!("{:064x}", index + 1);
            metadata.size_bytes = MAX_IMAGE_BYTES as u64;
        }
        assert!(matches!(
            validate_catalog_payload(
                &serde_json::to_vec(&sample_catalog(vec![oversized_pack])).unwrap()
            ),
            Err(ThemeErrorCode::CatalogInvalid)
        ));

        let second_pack = sample_remote_theme("winter", "12-01", "01-01");
        assert!(validate_catalog_payload(
            &serde_json::to_vec(&sample_catalog(vec![
                sample_remote_theme("october", "10-01", "11-01"),
                second_pack
            ]))
            .unwrap()
        )
        .is_ok());
    }

    #[test]
    fn annual_windows_are_half_open_and_leap_day_uses_march_first_when_needed() {
        let start = AnnualDate::parse("10-01").unwrap();
        let end = AnnualDate::parse("11-01").unwrap();
        assert!(is_in_annual_range(
            Date::from_calendar_date(2025, Month::October, 1).unwrap(),
            start,
            end
        ));
        assert!(!is_in_annual_range(
            Date::from_calendar_date(2025, Month::November, 1).unwrap(),
            start,
            end
        ));

        let leap_start = AnnualDate::parse("02-29").unwrap();
        let march_end = AnnualDate::parse("03-02").unwrap();
        let common_year_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        assert!(is_in_annual_range(common_year_date, leap_start, march_end));
        assert_eq!(
            upcoming_start(
                &ThemeContent {
                    id: "leap-theme".to_owned(),
                    name: "Leap theme".to_owned(),
                    description: "test".to_owned(),
                    start_month_day: "02-29".to_owned(),
                    end_month_day: "03-02".to_owned(),
                    revision: 1,
                    palettes: load_bundled_theme().unwrap().palettes,
                    artwork: BTreeMap::new(),
                },
                Date::from_calendar_date(2025, Month::February, 28).unwrap()
            ),
            Some(Date::from_calendar_date(2025, Month::March, 1).unwrap())
        );

        let eastern = UtcOffset::from_hms(-4, 0, 0).unwrap();
        let before_start = Date::from_calendar_date(2025, Month::September, 30)
            .unwrap()
            .with_hms(23, 59, 59)
            .unwrap()
            .assume_offset(eastern);
        let october_midnight = next_local_midnight_with_offset(before_start, eastern).unwrap();
        assert_eq!(
            october_midnight.date(),
            Date::from_calendar_date(2025, Month::October, 1).unwrap()
        );
        let after_end = Date::from_calendar_date(2025, Month::October, 31)
            .unwrap()
            .with_hms(23, 59, 59)
            .unwrap()
            .assume_offset(eastern);
        assert_eq!(
            next_local_midnight_with_offset(after_end, eastern)
                .unwrap()
                .date(),
            Date::from_calendar_date(2025, Month::November, 1).unwrap()
        );
        let october_theme = load_bundled_theme().unwrap();
        assert!(is_content_active(
            &october_theme,
            Date::from_calendar_date(2025, Month::October, 1).unwrap()
        ));
        assert!(!is_content_active(
            &october_theme,
            Date::from_calendar_date(2025, Month::November, 1).unwrap()
        ));
    }

    fn encode_test_png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![0; (width * height * 3) as usize])
                .unwrap();
        }
        bytes
    }

    #[test]
    fn png_validator_rejects_hash_correct_malformed_oversized_and_animated_images() {
        let malformed = b"not a png";
        let malformed_metadata = RemoteArtwork {
            url: "https://github.com/OrionAISystems/Lyra-Releases/releases/download/themes-october-v1/image.png".to_owned(),
            sha256: sha256_hex(malformed),
            size_bytes: malformed.len() as u64,
        };
        assert_eq!(
            validate_artwork_bytes(&malformed_metadata, malformed),
            Err(ThemeErrorCode::AssetsUnavailable)
        );

        let too_wide = encode_test_png(MAX_IMAGE_DIMENSION + 1, 1);
        let wide_metadata = RemoteArtwork {
            url: malformed_metadata.url.clone(),
            sha256: sha256_hex(&too_wide),
            size_bytes: too_wide.len() as u64,
        };
        assert_eq!(
            validate_artwork_bytes(&wide_metadata, &too_wide),
            Err(ThemeErrorCode::AssetsUnavailable)
        );

        for animated_chunk in [b"acTL".as_slice(), b"fcTL".as_slice(), b"fdAT".as_slice()] {
            let mut animated = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x00".to_vec();
            animated.extend_from_slice(animated_chunk);
            animated.extend_from_slice(&[0, 0, 0, 0]);
            assert!(png_has_animation_chunk(&animated));
            let metadata = RemoteArtwork {
                url: malformed_metadata.url.clone(),
                sha256: sha256_hex(&animated),
                size_bytes: animated.len() as u64,
            };
            assert_eq!(
                validate_artwork_bytes(&metadata, &animated),
                Err(ThemeErrorCode::AssetsUnavailable)
            );
        }
    }

    #[test]
    fn image_redirect_allowlist_and_hop_limit_are_enforced() {
        let release_asset =
            Url::parse("https://release-assets.githubusercontent.com/object").unwrap();
        let object_asset = Url::parse("https://objects.githubusercontent.com/object").unwrap();
        let wrong_host = Url::parse("https://evil.example/object").unwrap();
        let http_asset = Url::parse("http://release-assets.githubusercontent.com/object").unwrap();
        assert!(asset_redirect_is_allowed(&release_asset, 0));
        assert!(asset_redirect_is_allowed(&object_asset, 3));
        assert!(!asset_redirect_is_allowed(&release_asset, 4));
        assert!(!asset_redirect_is_allowed(&wrong_host, 0));
        assert!(!asset_redirect_is_allowed(&http_asset, 0));
    }

    #[test]
    fn bounded_response_body_rejects_an_oversized_chunked_stream() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\n12345\r\n0\r\n\r\n",
                )
                .unwrap();
        });
        let response = Client::builder()
            .redirect(Policy::none())
            .build()
            .unwrap()
            .get(format!("http://{address}/catalog"))
            .send()
            .unwrap();
        assert_eq!(
            bounded_response_body(response, 4),
            Err(ThemeErrorCode::AssetsUnavailable)
        );
        server.join().unwrap();
    }

    #[test]
    fn cache_without_an_authenticated_revision_can_bootstrap_after_quarantine() {
        let cache = TemporaryCache::new();
        let state = theme_cache_root(&cache.0).join("state.json");
        fs::create_dir_all(state.parent().unwrap()).unwrap();
        fs::write(&state, b"{not valid state").unwrap();

        assert!(matches!(
            recover_cache_pointer(&cache.0, &pinned_public_key_text().unwrap()),
            Ok(None)
        ));
        assert!(!state.exists());
        assert!(theme_cache_root(&cache.0)
            .read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("state.quarantine-")));
    }

    #[test]
    fn incomplete_staging_directory_never_becomes_active_cache_content() {
        let cache = TemporaryCache::new();
        let staging = theme_cache_root(&cache.0)
            .join("snapshots")
            .join(".staging-interrupted");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("catalog.json"), b"incomplete").unwrap();
        assert!(matches!(
            recover_cache_pointer(&cache.0, &pinned_public_key_text().unwrap()),
            Ok(None)
        ));
        assert!(!theme_cache_root(&cache.0).join("state.json").exists());
    }

    #[test]
    fn refresh_gate_does_not_queue_duplicate_catalog_work() {
        let gate = Mutex::new(());
        let guard = gate.lock().unwrap();
        let mut calls = 0;
        assert_eq!(try_with_refresh_gate(&gate, || calls += 1), None);
        assert_eq!(calls, 0);
        drop(guard);
        assert_eq!(try_with_refresh_gate(&gate, || calls += 1), Some(()));
        assert_eq!(calls, 1);
    }

    #[test]
    fn release_verification_key_is_read_from_tauri_updater_configuration() {
        let config: serde_json::Value = serde_json::from_str(TAURI_CONFIG).unwrap();
        let encoded = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        assert_eq!(
            pinned_public_key_text().unwrap(),
            String::from_utf8(STANDARD.decode(encoded).unwrap()).unwrap()
        );
    }

    #[test]
    fn catalog_fixture_from_environment_is_verified_by_production_pipeline() {
        let Some(fixture) = signed_fixture_from_environment() else {
            return;
        };
        let expected_invalid_signature =
            std::env::var("LYRA_THEMES_FIXTURE_EXPECT_INVALID").is_ok_and(|value| value == "1");
        let verified = verify_signed_catalog(&fixture.envelope, &fixture.public_key);
        if expected_invalid_signature {
            assert!(
                verified.is_err(),
                "tampered signed catalog must be rejected"
            );
            return;
        }

        let (payload, catalog) = verified.expect("signed catalog must pass production validation");
        assert!(payload.len() <= MAX_CATALOG_BYTES);
        if let Some(asset_dir) = fixture.asset_dir.as_deref() {
            let expected_invalid_asset = std::env::var("LYRA_THEMES_FIXTURE_EXPECT_INVALID_ASSET")
                .is_ok_and(|value| value == "1");
            let assets = fixture_assets(&catalog, asset_dir, None);
            if expected_invalid_asset {
                assert!(
                    assets.is_err(),
                    "tampered PNG must fail hash and decode checks"
                );
            } else {
                assets.expect("fixture artwork must pass production hash/PNG validation");
            }
        }
    }

    #[test]
    fn cache_recovers_and_repairs_signed_fixture_offline() {
        let Some(fixture) = signed_fixture_from_environment() else {
            return;
        };
        if std::env::var("LYRA_THEMES_FIXTURE_EXPECT_INVALID").is_ok()
            || std::env::var("LYRA_THEMES_FIXTURE_EXPECT_INVALID_ASSET").is_ok()
        {
            return;
        }
        let asset_dir = fixture
            .asset_dir
            .as_deref()
            .expect("cache fixture requires the prepared asset directory");
        let (payload, catalog) = fixture_catalog(&fixture);
        let selected_ids = relevant_theme_ids(&catalog, local_now().date());
        assert!(!selected_ids.is_empty());
        let assets = fixture_assets(&catalog, asset_dir, Some(&selected_ids))
            .expect("selected fixture assets should validate");

        let cache = TemporaryCache::new();
        let state_path = theme_cache_root(&cache.0).join("state.json");
        fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        fs::write(&state_path, b"corrupt pointer").unwrap();
        let key = fixture.public_key.as_str();
        let floor = recover_cache_pointer(&cache.0, key)
            .expect("cache recovery should permit a fresh signed bootstrap");
        assert!(floor.is_none());

        commit_snapshot(
            &cache.0,
            &fixture.envelope,
            &payload,
            &catalog,
            &selected_ids,
            &assets,
            floor.as_ref(),
        )
        .expect("complete signed catalog snapshot should commit");
        let loaded = load_cached_snapshot_with_key(&cache.0, key)
            .expect("new snapshot should verify offline")
            .expect("new snapshot should be available");
        assert_eq!(loaded.0, catalog.revision);

        let pointer = recover_cache_pointer(&cache.0, key)
            .expect("signed catalog revision should remain a rollback floor")
            .expect("snapshot pointer should remain present");
        let damaged_hash = catalog
            .themes
            .iter()
            .filter(|theme| selected_ids.iter().any(|id| id == &theme.id))
            .flat_map(|theme| theme.artwork.values())
            .next()
            .expect("selected theme should have artwork")
            .sha256
            .clone();
        let damaged_path = theme_cache_root(&cache.0)
            .join("snapshots")
            .join(&pointer.active_snapshot)
            .join("assets")
            .join(format!("{damaged_hash}.png"));
        let mut damaged_bytes = fs::read(&damaged_path).unwrap();
        damaged_bytes[0] ^= 0xff;
        fs::write(&damaged_path, &damaged_bytes).unwrap();
        assert!(matches!(
            load_cached_snapshot_with_key(&cache.0, key),
            Err(ThemeErrorCode::CacheUnavailable)
        ));

        let recovered_floor = recover_cache_pointer(&cache.0, key)
            .expect("signed catalog should still authenticate the revision")
            .expect("revision floor should recover despite damaged artwork");
        commit_snapshot(
            &cache.0,
            &fixture.envelope,
            &payload,
            &catalog,
            &selected_ids,
            &assets,
            Some(&recovered_floor),
        )
        .expect("same signed revision should repair a corrupt image snapshot");
        let repaired_pointer = read_cache_pointer(&cache.0)
            .expect("repaired pointer should be readable")
            .expect("repaired pointer should be present");

        // Simulate a stale same-revision snapshot left behind by interrupted or failed cleanup.
        let stale_name = format!(
            "rev-{}-{}-{}",
            catalog.revision,
            &sha256_hex(&payload)[..16],
            "0".repeat(32)
        );
        let stale_snapshot = theme_cache_root(&cache.0)
            .join("snapshots")
            .join(stale_name);
        fs::create_dir_all(stale_snapshot.join("assets")).unwrap();
        fs::write(stale_snapshot.join("catalog.json"), &fixture.envelope).unwrap();
        let manifest_bytes = serde_json::to_vec(&SnapshotManifest {
            selected_theme_ids: selected_ids.clone(),
        })
        .unwrap();
        fs::write(stale_snapshot.join("manifest.json"), manifest_bytes).unwrap();
        for (hash, bytes) in &assets {
            let bytes = if hash == &damaged_hash {
                &damaged_bytes
            } else {
                bytes
            };
            fs::write(
                stale_snapshot.join("assets").join(format!("{hash}.png")),
                bytes,
            )
            .unwrap();
        }

        let recovered_pointer = recover_cache_pointer(&cache.0, key)
            .expect("same-revision recovery should preserve the active snapshot")
            .expect("repaired snapshot pointer should remain present");
        assert_eq!(
            recovered_pointer.active_snapshot,
            repaired_pointer.active_snapshot
        );
        let repaired = load_cached_snapshot_with_key(&cache.0, key)
            .expect("repaired snapshot should verify offline")
            .expect("repaired snapshot should be available");
        assert_eq!(repaired.0, catalog.revision);

        let mut older_catalog = catalog.clone();
        older_catalog.revision = older_catalog.revision.saturating_sub(1);
        assert_eq!(
            commit_snapshot(
                &cache.0,
                &fixture.envelope,
                &payload,
                &older_catalog,
                &selected_ids,
                &assets,
                Some(&recovered_floor),
            ),
            Err(ThemeErrorCode::RollbackDetected)
        );

        let mut changed_payload_same_revision = payload.clone();
        changed_payload_same_revision.push(b' ');
        assert_eq!(
            commit_snapshot(
                &cache.0,
                &fixture.envelope,
                &changed_payload_same_revision,
                &catalog,
                &selected_ids,
                &assets,
                Some(&recovered_floor),
            ),
            Err(ThemeErrorCode::RollbackDetected)
        );
    }

    #[test]
    #[ignore = "requires the published signed theme channel and network access"]
    fn live_published_theme_channel() {
        let (envelope, payload, catalog) =
            fetch_catalog().expect("published catalog should verify using the pinned key");
        let selected_ids = relevant_theme_ids(&catalog, local_now().date());
        assert!(
            !selected_ids.is_empty(),
            "published catalog should have a current or upcoming theme"
        );
        let assets = download_artwork(&catalog, &selected_ids)
            .expect("published artwork should pass production redirect/hash/PNG checks");
        let cache = TemporaryCache::new();
        commit_snapshot(
            &cache.0,
            &envelope,
            &payload,
            &catalog,
            &selected_ids,
            &assets,
            None,
        )
        .expect("published content should stage atomically in the temporary cache");
        let loaded = load_cached_snapshot_with_key(&cache.0, &pinned_public_key_text().unwrap())
            .expect("staged published content should reverify offline")
            .expect("staged published content should be active in the cache");
        assert_eq!(loaded.0, catalog.revision);
        assert_eq!(loaded.2.len(), selected_ids.len());
    }
}

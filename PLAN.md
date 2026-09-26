# Lyra v0.5.0 — Local History and Scratchpad

## Status and delivery boundary

This plan records the product decisions and implementation sequence for v0.5.0. The release was published on 2026-09-26; see [docs/ACCEPTANCE-V0.5.md](docs/ACCEPTANCE-V0.5.md) for build, review, workflow, and public-artifact evidence. The current published version is v0.5.0.

The implementation and local review were completed before release. After the user reviewed the build and approved release, the existing updater-signed tag workflow published v0.5.0. Hardware-backed, installed-updater, and other unobserved acceptance cases remain explicitly pending in the acceptance record.

## Product contract

### History

- Transcript History is **Off by default**. No earlier v0.4 transcript can be recovered or backfilled. When enabled, save final, processed text from normal dictation, including a result whose automatic paste failed. Do not save partials, failed recognitions without final text, cancelled captures, raw audio, or cloud request payloads.
- Retention choices: Off, 7 days, 30 days, 90 days, 1 year, Forever. Changing to a shorter period prunes already saved entries immediately. Switching to Off stops future saves but leaves existing entries visible; they continue expiring under the last selected non-Off period. Forever does not expire. Clear transcript history removes all History entries regardless of the selected period, without clearing Scratchpad or Insights.
- The History page groups entries by local calendar day and shows time, target application, word count, capture duration, and actual recognition engine/backend. Selecting an item shows its full transcript, timestamp, target app, duration, engine, latency, and paste outcome. Actions: Copy, Paste again, Send to Scratchpad, Delete. Promotion creates an independent Scratchpad copy and leaves the History item in place.
- History is passive. Scratchpad captures do not create History entries, even when History is enabled. Deleting a History item or clearing History removes saved transcript content, not aggregate Insights or its independent metadata ledger.

### Scratchpad

- Scratchpad is an intentional speech inbox, not a general notes system. Give it a separate sidebar page, a visible Record control, and a configurable global shortcut. Both capture routes use the existing push-to-talk/toggle activation setting, microphone, engine, language, fallback, and text processing pipeline.
- A Scratchpad capture saves the final processed text locally and does **not** automatically paste or copy it. It can start while Lyra is foreground; it does not need an external target. Show capture, processing, success, and failure states, and open/show the saved entry after success.
- Entries remain until individually deleted. Pinning keeps an entry at the top; otherwise show newest first. Each entry has Copy, Paste into current app, Pin/Unpin, and Delete. No full notes editor, tags, folders, search, sync, or automatic expiry in v0.5.0. Promoting a History item creates the same kind of Scratchpad entry.
- Scratchpad recordings count in overall Insights sessions, words, duration, engine, and latency. They do not count as paste attempts or paste failures. Promotion from History adds no new usage.

### Paste and privacy

- From either page, Paste targets the most recent **non-Lyra** foreground window, not the app stored on the History record. Track that window without persisting a full path or title. Refocus it, verify the window/process identity immediately before injection, and use the existing app profile and clipboard recovery flow. If no safe target exists or Windows refuses focus/injection, copy the text and show a clear, recoverable error; never paste into an unverified window.
- Store transcript and Scratchpad text only on the local machine and protect it at rest with Windows user-bound encryption (DPAPI). Keep encryption and decryption inside the Rust storage boundary. Do not emit full saved lists or transcript text in broad state snapshots, logs, telemetry, diagnostics, or usage JSON. Return text only for an explicitly requested item/action. Document that Windows-user-bound encrypted data is not portable to another account/machine without a future export feature.
- Preserve the current default privacy posture: no transcript persistence until History is explicitly enabled, except when the user deliberately records to Scratchpad. Clipboard behavior remains explicit for saved-item actions. Confirmation is required for Clear transcript history; individual Delete needs an undo or a concise confirmation pattern consistent with the finished UI.

## Data ownership and migration

1. Keep `usage.rs` as the metadata and aggregate Insights ledger. Remove its 90-day daily cap so daily aggregates persist indefinitely. Replace its 200-record session cap with a rolling one-year age policy. Prune old metadata on load/write and keep lifetime totals intact. Preserve v0.4 data during schema migration, including its existing `detailedSince` semantics. The Insights range control should expose longer ranges/all time and remain usable with years of daily data; monthly/yearly compaction is unnecessary in v0.5.0.
2. Add separate Rust stores for History and Scratchpad under `%LOCALAPPDATA%\Lyra\`, with distinct retention and deletion operations. Persist a stable item ID and a metadata snapshot with each encrypted text item. The snapshot is essential: a Forever History entry remains intelligible after its corresponding usage session metadata passes the one-year boundary. Avoid coupling an item's survival to `usage.json`.
3. Version the new store formats and settings schema. Use atomic file replacement, bounded reads, strict deserialization, predictable ordering, and clear handling of corrupt records or DPAPI decryption failure. Do not silently convert a failed encrypted write into plaintext or report a save as successful. Ensure temporary files cannot retain plaintext. No new database dependency is justified for this small local dataset; revisit only if measured load or update cost requires it.
4. Add a History retention setting and the last non-Off retention value. Settings migration defaults History to Off and leaves all existing settings, shortcuts, personalization, and usage intact. Register the Scratchpad shortcut through the existing duplicate detection and rollback path, including conflict handling with the primary and recovery shortcuts.
5. Represent session destination as immutable request-scoped state (`dictation` or `scratchpad`) shared by shortcut, bar/UI, capture coordinator, and completion. Keep one microphone session at a time. Route only a final processed result to either paste/history or Scratchpad; do not duplicate recognition logic. Make capture cancellation, failed recognition, save failure, and update-install blocking visible and recoverable. For normal dictation, a History write failure must not turn a successful paste into a failed dictation; surface the local-save failure separately.

## UI and integration

1. Add History and Scratchpad as separate destinations in the existing settings shell. Keep the pinned sidebar and content-only scrolling at desktop and narrow widths. Add an understandable empty state for History Off, History On with no entries, expired/cleared History, and empty Scratchpad.
2. Put Transcript history retention and Clear transcript history controls in the appropriate settings area, with precise privacy copy. Make clear that turning Off stops new saves while existing entries continue under their previous retention. Add the Scratchpad shortcut beside the existing shortcut configuration and show collision/registration feedback.
3. Extend the Rust command/TypeScript bridge with paginated list summaries, item detail, delete, clear, promote, pin, copy, paste, start Scratchpad capture, and status refresh. Keep commands intent-level and validate IDs and state in Rust. Do not make the frontend the source of truth for retention or encryption.
4. Ensure bar feedback distinguishes the capture destination. The same activation gesture must stop the session it started, even if a setting or window changes mid-capture. Existing last-transcript recovery remains intact for normal dictation; Scratchpad's save result should be visible in its own page.
5. Update README, `ARCHITECTURE.md`, `BUILD-STAGE.md`, privacy/setup notes where relevant, and add `docs/ACCEPTANCE-V0.5.md`. Replace statements that categorically prohibit saved transcript history with the new opt-in contract.

## Implementation sequence and verification

1. **Storage and migration:** settings schema, encryption helper, separate stores, independent retention, usage migration. Unit tests cover default Off, opt-in writes, Off with continued expiry, all retention values, shortening a period, Forever, clear/delete isolation, DPAPI failure/corruption, one-year metadata pruning, and preservation of lifetime totals.
2. **Capture routing:** request-scoped destination, shared recognition/processing, Scratchpad shortcut and UI start/stop, one-session arbitration, actual backend metadata, write-failure behavior. Focused Rust tests cover success, paste failure, no speech, cancellation, targetless Scratchpad, shortcut collision/rollback, and an in-flight settings change.
3. **Saved-item actions and UI:** list/detail/pagination, copy/paste/promotion/pin/delete/clear, privacy and error states, keyboard behavior, responsive layout. Verify that text does not leak into usage, logs, diagnostics, or broad snapshots. Exercise real click targets and focus behavior in the rendered app.
4. **Local gate:** synchronize source version files to 0.5.0 and draft its canonical changelog entry before building, so the reviewed build identifies itself correctly. Run repository checks (`npm run typecheck`, `npm test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run verify:version`, `npm run verify:changelog`, `npm run verify:release-config`). Build and run the local dev app; record manual results in the v0.5 acceptance sheet for normal dictation, Scratchpad via shortcut and UI, cross-app paste, target loss, restart persistence, retention transitions, encryption-at-rest inspection, and narrow/desktop UI states. Keep hardware-only cases explicitly pending if they cannot be observed.
5. **Review checkpoint (complete):** the user reported that basic initial tests worked and explicitly approved the v0.5.0 release. Detailed unobserved behavior remains pending in the acceptance sheet.
6. **Release (complete):** finalized the changelog, pushed the v0.5.0 tag, passed the tagged GitHub Actions workflow, and inspected the public installer digest, updater signature asset and manifest, and mirrored changelog. The installer is not Authenticode-signed because the optional certificate is not configured. Installed-updater and hardware acceptance remain pending until directly observed.

## Acceptance criteria

- Fresh and migrated installs have History Off with no transcript text persisted by ordinary dictation until explicitly enabled.
- Enabled History displays the exact final processed text and requested metadata; failed paste remains recoverable. Retention, Off, per-item Delete, and Clear behave independently of Scratchpad and Insights.
- Scratchpad records via both configured shortcut and UI without pasting or changing the clipboard, persists across restart, supports pin/copy/paste/delete, and remains independent of History retention.
- Saved text is Windows-user encrypted at rest; audio, partials, logs, diagnostics, and `usage.json` contain no transcript text.
- Daily Insights aggregates survive beyond 90 days; metadata-only sessions follow one-year retention; lifetime totals do not regress.
- Paste actions reach the verified most recent external app or return a recoverable copied-text error. Existing dictation, app profiles, recovery shortcuts, and update guards continue to work.
- The local dev build is reviewed before the release stage begins.

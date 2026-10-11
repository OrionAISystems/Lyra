# v0.5.10 acceptance

Scope: restore explicit CUDA, Vulkan and CPU runtime setup in Audio & Engine,
and center the Insights empty state inside its plot. Windows startup controls
remain in Appearance.

## Automated and rendered evidence

- Frontend typecheck, tests and production build passed.
- Runtime UI regression verifies a fresh device without NVIDIA detection defaults
  to CPU, all three options are present, CPU/Vulkan selections pass the correct
  variant to setup, and controls are disabled during installation.
- Runtime regression selects already verified CUDA/Vulkan variants without
  invoking download progress or replacing fixture DLL bytes.
- Playwright inspected the app at `http://127.0.0.1:1420` (title Lyra). Runtime
  dropdown opened and keyboard ArrowUp/Enter selected CPU. Desktop preview cannot
  invoke Tauri downloads; the command contract is covered by the UI regression.
- Empty Insights with bundled October artwork was rendered at 1100x680, 800x700
  and 390x844. Screenshots and bounding-box checks confirmed centered artwork/text
  and a plot contained inside the chart card. Populated Insights remains on the
  same bar-stack path.
- No framework overlay or app console errors; browser preview requested a
  missing favicon (404). Browser plugin was unavailable, so Playwright CLI was used.

## Remaining hardware acceptance

- Runtime installation and CPU microphone dictation on the reported AMD laptop
  remain unverified. No runtime/model downloads were performed on this machine.
- Vulkan compatibility depends on the device's graphics driver; CPU is available
  when compatible GPU acceleration is unavailable.
- Installed v0.5.9-to-v0.5.10 updater acceptance remains unverified.

## Release gates and publication

Final Rust, fixture, main CI, tag publication and public artifact results are
recorded below once completed.

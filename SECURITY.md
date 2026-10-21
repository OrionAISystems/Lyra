# Security policy

Security fixes target the latest stable version. Older releases may remain
available for reference but are not separately maintained.

## Report a vulnerability privately

Use [GitHub private vulnerability reporting](https://github.com/OrionAISystems/Lyra/security/advisories/new)
when available. If it is unavailable, email the maintainer at
<siphrcodez@gmail.com> with the subject `Lyra security report`. Do not open a
public issue containing an exploit, personal transcript, credential, or signing
material. Include the affected version, Windows version, reproduction steps,
impact, and a minimal invented/sanitized example. No response-time guarantee or
bug bounty is offered.

## Boundaries to preserve

- Raw microphone audio stays in memory; History is opt-in and saved text uses
  user-bound Windows DPAPI. Never log audio, transcripts, clipboard text, or keys.
- `LocalOnly` never falls back to cloud. The default local-first cloud-fallback
  route can send audio to OpenAI after local failure when a credential is stored.
- Stored credentials use Windows Credential Manager. Foreground target, focus,
  cancellation, and clipboard-sequence checks guard insertion and recovery.
- Runtime/model downloads use pinned checksums; app updates and seasonal artwork
  use signed, separate feeds. Public verification keys are intentional; private
  signing keys and publication credentials must remain outside Git.

These are security requirements, not a claim that every possible vulnerability
has been excluded. See [ARCHITECTURE.md](ARCHITECTURE.md) for ownership and data flow.

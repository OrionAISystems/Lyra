# Dictation text formatting

All recognition routes share the domain text-processing pipeline (`process_transcription_text` and its insertion-context variant): line-ending normalization → self-correction → spoken formatting/quantities/contextual casing → corrections → snippets → insertion cleanup. Formatting obeys the existing **Spoken formatting** switch and profile flag. Self-correction has its own preference; later personalization wins. No model call or external service is used for formatting.

## Punctuation and casing

Spoken comma/period commands own adjacent recognizer punctuation, including marks separated by spaces/tabs: `report period.` → `report.`, `Hello. , period next` → `Hello. Next`. They do not cross line breaks when removing marks. Literal punctuation without a spoken cue remains intact.

A spoken period capitalizes the following lowercase word, including after opening quotes or a line break. Mixed-case spellings such as `eBay`, `d’Artagnan` and `i-Ready` survive. A bare newline does not recase list items. Self-correction preserves the replaced clause's initial uppercase style, including after opening delimiters.

### Continuing text at the insertion cursor

For automatic Dictation paste with Spoken formatting and the target app's formatting profile enabled, Lyra queries the focused Windows accessibility TextPattern immediately before final text processing/paste. It requests a range of 128 characters before the actual caret, including text typed manually. Returned text is bounded to 129 UTF-16 units; the extra unit is used only to reject oversized or incorrectly expanded provider ranges. It does not use the previous Lyra transcript. The query never moves the selection, sends keystrokes, reads the clipboard, or retrieves the entire document.

If the preceding context indicates an unfinished sentence, a recognized initial titlecase word becomes lowercase: `typed text ` + `And this continues` → `typed text and this continues`. Only the opening word changes; later sentences retain their casing. Period/question/exclamation marks, ellipses and paragraph breaks retain the recognized opening case, looking past spaces, tabs, nonbreaking spaces and surrounding quote/bracket marks. An existing letter/digit immediately before insertion uses the shared word-spacing cleanup.

`I` and its common contractions, acronyms, mixed-case spellings, enabled capitalized Words entries, and matching Corrections/Snippet triggers preserve their casing. Corrections and snippets still own the final text. A titlecase proper name such as `John` cannot be distinguished deterministically from recognizer-added sentence capitalization; add it to Words or Corrections to preserve it at the start of a continuation. No change is made to Scratchpad, clipboard-only output, replay of saved text, or formatting-disabled captures.

Context reading uses one dedicated COM worker with a bounded queue, provider timeouts, and a 250 ms caller budget. Only the preceding character and continuation flag return to the processing flow; surrounding text remains transient on the worker and is never logged, saved, exported or sent to recognition/cloud providers. Foreground HWND/process, focused element/process, password status and caret selection are checked; focus/selection changes, selected text, unsupported controls, overexpanded provider ranges, and timeouts fall back to recognized casing. Custom editors or elevated apps may not expose a usable TextPattern; support is verified per target rather than assumed.

See [cursor-context acceptance](ACCEPTANCE-CURSOR-CONTEXT.md) for the synthetic native fixture and pending microphone/app checks.

## Canonical English quantities

Convert clear quantities and standalone multiword cardinal/decimal phrases. Single ambiguous words remain prose: `one more thing`, `take five`, `one of a kind`. Units use a bounded allowlist of common counts, measurements, durations and storage abbreviations; arbitrary nouns are not inferred as quantities.

| Recognized text | Final output |
| --- | --- |
| `twenty five` / `one hundred and five` | `25` / `105` |
| `a hundred boxes` / `five boxes` | `100 boxes` / `5 boxes` |
| `five GB` | `5 GB` |
| `minus twelve point five degrees` | `-12.5 degrees` |
| `twelve point five percent` / `per cent` | `12.5%` |
| `forty five dollars` / `$45` | `$45` |
| `one dollar and five cents` | `$1.05` |
| `$1.00 and five cents` | `$1.05` |
| `£1 and one penny` | `£1.01` |
| `costs five pounds` / `five pounds sterling` | `costs £5` / `£5` |
| `weighs five pounds` | `weighs 5 pounds` |

Cardinals use ordinary English grammar through trillions with checked integer arithmetic. Attached hyphens and an optional cardinal `and` are supported; mixed forms such as `12 point five` and `one hundred 5` are parsed together. Articles require a unit/currency context. Decimal fractional digits support `zero`/`oh`, up to twelve places; decimal magnitudes such as `two point five million` are not expanded.

Dollars/euros use `$`/`€`. Pounds require an existing `£`, `sterling`, or a nearby monetary cue; weight stays in pounds. Monetary lookback is bounded to four tokens through recognized short connecting words and stops at punctuation, newlines or unrelated content.

Money removes grouping commas and all-zero fractions, pads a single nonzero fractional digit, and retains higher precision without rounding: `$1,234.50` → `$1234.50`, `$45.00` → `$45`, `$12.5` → `$12.50`, `$0.0050` → `$0.005`. Integer major amounts can combine `and` plus 0–99 cents/pence (including singular cent/penny). Standalone minor units do not imply a currency. Written numerals accept eighteen whole and twelve fractional digits.

## Boundaries and opt-out

This is a bounded prose formatter, not a complete URI, code, calendar or financial-expression parser. Known identifier delimiters, written dates, titles, repeated digit sequences, malformed recognized cardinal spans and incompatible recognized currency annotations have preservation checks. These checks are best effort for arbitrary malformed or contradictory ASR text; they do not promise exhaustive recovery of every combination. Formatting may normalize a recognized amount while leaving unrecognized trailing prose untouched.

Number phrases do not join across newlines or spaced subtraction. Capitalized title tokens protect names such as `Twenty One Pilots` and `Five Seconds of Summer`; uppercase known short units still convert. Ordinary cardinals beside month words still become digits (`October twenty five` → `October 25`), without rearranging date layout or interpreting ordinals.

The chosen standalone-number policy can convert lowercase names/idioms that resemble valid cardinals. Telephone/PIN layout, ordinals, fractions, scientific notation, non-English number grammars and currency conversion are outside this pass. Use formatting off for arbitrary literal/code-oriented dictation; profile overrides and personalization remain available.

Tests cover supported grammar, punctuation/casing, opt-out, personalization, preservation boundaries and benchmark acceptance. See [results and hardware limits](../benchmarks/transcription/CLEANUP-RESULTS.md).

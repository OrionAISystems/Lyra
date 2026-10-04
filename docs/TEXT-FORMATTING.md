# Dictation text formatting

All recognition routes share `process_transcription_text`: line-ending normalization → self-correction → spoken formatting/quantities → corrections → snippets → insertion cleanup. Formatting obeys the existing **Spoken formatting** switch and profile flag. Self-correction has its own preference; later personalization wins. No new UI, dependency, model call or service is added.

## Punctuation and casing

Spoken comma/period commands own adjacent recognizer punctuation, including marks separated by spaces/tabs: `report period.` → `report.`, `Hello. , period next` → `Hello. Next`. They do not cross line breaks when removing marks. Literal punctuation without a spoken cue remains intact.

A spoken period capitalizes the following lowercase word, including after opening quotes or a line break. Mixed-case spellings such as `eBay`, `d’Artagnan` and `i-Ready` survive. A bare newline does not recase list items. Self-correction preserves the replaced clause's initial uppercase style, including after opening delimiters.

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

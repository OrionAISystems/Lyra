# Dictation text formatting

All recognition routes share `process_transcription_text`: line-ending normalization → explicit self-correction → spoken formatting and quantities → corrections → snippets → insertion cleanup. Formatting obeys the existing **Spoken formatting** switch and the app profile's `formatting_enabled` flag. Disabling formatting leaves number words and formatting cues as recognized. Self-correction has its existing independent preference. These changes add no UI, schema version, dependency, model call or external service.

## Punctuation and sentence casing

Spoken `comma` and `period` commands own adjacent punctuation supplied by the recognizer, including conflicting marks before the cue: `Hello, period.` becomes `Hello.` and `Hello. comma,` becomes `Hello,`. `report period.` becomes `report.`; `Hello, comma, Orion` becomes `Hello, Orion`. Literal punctuation without a spoken cue is preserved, including ellipses, decimal points, versions and URLs. Line-break commands continue to remove recognizer punctuation attached to the command itself.

A spoken period capitalizes the following lowercase word, including across an explicit newline/paragraph and opening quotes. Mixed-case words such as `iPhone` and `eBay` retain their casing. A bare newline does not capitalize list items, and ordinary recognized sentences are not globally recased. Explicit self-correction preserves the initial uppercase style of the clause it replaces: `Send ... scratch that send ...` produces `Send ...`, also when an opening quote or bracket precedes the discarded clause's first word. User corrections and snippets run later, so their intentional spelling/casing wins.

## Canonical English quantities

The chosen policy converts standalone **multiword** English cardinal/decimal phrases and single number words with known quantity units. Ambiguous single words remain prose: `one more thing`, `take five`, `one of a kind`. The unit allowlist covers common counts, durations, lengths, weights, volumes, temperatures and storage quantities; it does not infer every noun as a count unit.

| Recognized text | Final output |
| --- | --- |
| `twenty five` | `25` |
| `one hundred and five` | `105` |
| `a hundred boxes` | `100 boxes` |
| `five boxes` | `5 boxes` |
| `twelve point zero five kilograms` | `12.05 kilograms` |
| `point five` | `0.5` |
| `minus twelve point five degrees` | `-12.5 degrees` |
| `twelve point five percent` / `per cent` | `12.5%` |
| `forty five dollars` / `$45` | `$45` |
| `one dollar and five cents` | `$1.05` |
| `$1 and five cents` / `£1 and five pence` | `$1.05` / `£1.05` |
| `two euros and ninety nine cents` | `€2.99` |
| `costs five pounds` / `five pounds sterling` | `costs £5` / `£5` |
| `weighs five pounds` | `weighs 5 pounds` |

Spoken cardinals use ordinary English grammar through trillions, with descending scales and checked integer arithmetic. Hyphenated tens and optional `and` within a cardinal are supported. `A hundred`/`a million` require a unit or explicit currency context; bare article phrases remain prose, preserving `one in a million`. Mixed recognizer forms such as `12 point five` and `one hundred 5 boxes` are parsed as complete phrases. Repeated digit words such as `one two three four`, malformed scale sequences, and ambiguous `twenty four seven` are preserved rather than partially rewritten. Decimal fractional parts use individual digits, including `zero`/`oh`, up to twelve places. Decimal magnitudes such as `two point five million` are not expanded.

Explicit dollars/euros become `$`/`€`. `Pounds` is ambiguous with weight, so `£` requires an existing pound symbol, `sterling`, or a nearby monetary cue such as `costs`, `pay`, or `price`. The parser does not infer a country-specific dollar denomination or convert currencies. Named denominations and unsupported locale formats are not guessed.

Currency rendering removes grouping commas, removes an all-zero fractional part, and pads a nonzero one-digit fraction to two digits: `$1,234.50` → `$1234.50`, `$45.00` → `$45`, `$12.5` → `$12.50`. Higher precision is retained, without rounding (`$0.0050` → `$0.005`). Ordinary measured decimals preserve fractional precision. Major-unit amounts can combine `and` plus 0–99 cents/pence; standalone cents do not imply a currency. Written numerals accept up to eighteen whole digits and twelve fractional digits. Invalid grouping and conflicting currency annotations remain untouched.

## Ambiguity and opt-out boundaries

Identifier-like adjacency, existing versions, URLs, written dates, repeated digit sequences, mixed-case names and non-English number words are protected by deterministic checks. Attached dotted components remain literal (`twenty-five.com`, `version twenty five.1`), while a terminal sentence period permits conversion (`twenty five.` → `25.`). Hyphens join number words only when attached on both sides; spaced subtraction is not combined into a cardinal. Number phrases and monetary context never join across a newline. A following capitalized name token protects titles such as `Twenty One Pilots`, including titles whose next word is also a unit (`Five Seconds of Summer`, `Seven Pounds`, `One Hundred Years of Solitude`). Lowercase unit quantities still convert. Conflicting currency names remain literal whether the first annotation is a symbol or a named denomination (`$five pounds`, `five dollars euros`).

Known short unit abbreviations in uppercase remain quantity context (`five GB` → `5 GB`, `twelve MB` → `12 MB`), rather than title signals. Minor-currency conflicts are checked before committing any part of the amount: `one dollar and five pence` and `$1 and five cents euros` remain wholly literal. Comma- or newline-separated denominations belong to separate contexts.

Converting standalone multiword numbers still carries the explicitly chosen tradeoff: a lowercase name or idiom that is also a valid English number phrase can be converted. The formatter cannot recover a name that a recognizer already emitted as digits. Calendar/date layout, telephone/PIN formatting, ordinals, fractions, scientific notation and non-English grammars are outside this pass. Disable spoken formatting for literal/code-oriented dictation; profile overrides and later personalization remain available.

## Regression evidence

Domain tests cover command punctuation ownership, sentence/self-correction casing, literal punctuation preservation, formatting opt-out, personalization order and idempotent processing. Quantity tests cover numeric grammar, decimals, exact currency rendering, signs, percentages, weight/currency ambiguity and preservation boundaries. Synthetic fixture replays go through the installed recognizers and this same cleanup stage. See [the cleanup benchmark evidence](../benchmarks/transcription/CLEANUP-RESULTS.md) for measured results and hardware limitations.

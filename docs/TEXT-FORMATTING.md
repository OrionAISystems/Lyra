# Dictation text formatting

All recognition routes share `process_transcription_text`: line-ending normalization → explicit self-correction → spoken formatting and quantities → corrections → snippets → insertion cleanup. Formatting obeys the existing **Spoken formatting** switch and the app profile's `formatting_enabled` flag. Disabling formatting leaves number words and formatting cues as recognized. Self-correction has its existing independent preference. These changes add no UI, schema version, dependency, model call or external service.

## Punctuation and sentence casing

Spoken `comma` and `period` commands own adjacent punctuation supplied by the recognizer, including conflicting marks before the cue: `Hello, period.` becomes `Hello.` and `Hello. comma,` becomes `Hello,`. `report period.` becomes `report.`; `Hello, comma, Orion` becomes `Hello, Orion`. Literal punctuation without a spoken cue is preserved, including ellipses, decimal points, versions and URLs. Line-break commands continue to remove recognizer punctuation attached to the command itself.

A spoken period capitalizes the following lowercase word, including across an explicit newline/paragraph and opening quotes. Mixed-case words such as `iPhone` and `eBay` retain their casing. A bare newline does not capitalize list items, and ordinary recognized sentences are not globally recased. Explicit self-correction preserves the initial uppercase style of the clause it replaces: `Send ... scratch that send ...` produces `Send ...`, also when an opening quote or bracket precedes the discarded clause's first word. User corrections and snippets run later, so their intentional spelling/casing wins.

Word-internal apostrophes and hyphens participate in the mixed-case safeguard: `d’Artagnan` and `i-Ready` keep their spelling after a spoken period or self-correction. Ordinary lowercase hyphenated words still gain sentence casing.

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

Currency rendering removes grouping commas, removes an all-zero fractional part, and pads a nonzero one-digit fraction to two digits: `$1,234.50` → `$1234.50`, `$45.00` → `$45`, `$12.5` → `$12.50`. Higher precision is retained, without rounding (`$0.0050` → `$0.005`). Ordinary measured decimals preserve fractional precision. Integer major-unit amounts (including all-zero decimal fractions) can combine `and` plus 0–99 cents/pence; singular `cent` and `penny` are supported with the same currency compatibility checks. For example, `$1.00 and five cents` → `$1.05` and `£1 and one penny` → `£1.01`. Standalone cents do not imply a currency. Written numerals accept up to eighteen whole digits and twelve fractional digits. Invalid grouping and conflicting currency annotations, including adjacent symbols such as `five dollars £`, remain untouched.

## Ambiguity and opt-out boundaries

Identifier-like adjacency, existing versions, URLs, written dates, repeated digit sequences, mixed-case names and non-English number words are protected by deterministic checks. Attached dotted components remain literal (`twenty-five.com`, `version twenty five.1`), while a terminal sentence period permits conversion (`twenty five.` → `25.`). Hyphens join number words only when attached on both sides; spaced subtraction is not combined into a cardinal. Number phrases and monetary context never join across a newline. A following capitalized name token protects titles such as `Twenty One Pilots`, including titles whose next word is also a unit (`Five Seconds of Summer`, `Seven Pounds`, `One Hundred Years of Solitude`). Lowercase unit quantities still convert. Conflicting currency names remain literal whether the first annotation is a symbol or a named denomination (`$five pounds`, `five dollars euros`).

Known short unit abbreviations in uppercase remain quantity context (`five GB` → `5 GB`, `twelve MB` → `12 MB`), rather than title signals. Minor-currency conflicts are checked before committing any part of the amount: `one dollar and five pence` and `$1 and five cents euros` remain wholly literal. Comma- or newline-separated denominations belong to separate contexts.

Repeated adjacent currency annotations are malformed and remain literal (`five dollars dollars`, `five euros €`, `one dollar and five cents dollars`), avoiding a redundant intermediate result that changes again on another formatting pass. A single matching symbol/name pair such as `$five dollars` still canonicalizes to `$5`.

The same preservation rule covers repeated minor units (`one dollar and five cents cents`). Commas, semicolons, exclamation marks and question marks allow following quantities even without a space (`first,five boxes` → `first,5 boxes`). Dots and colons retain their ambiguous identifier-prefix protection; the formatter does not globally insert missing spaces.

An attached structured prefix (URL/domain/path or identifier markers) keeps those same punctuation characters literal, preserving query/path components such as `https://x.test?twenty-five` and `https://x.test/path;twenty-five`.

Attached query/fragment/assignment suffixes are protected too (`twenty-five?mode=full`, `twenty-five#section`); a terminal question mark still permits standalone number conversion. Complete unsupported minor clauses (including values outside 0–99 or noninteger major amounts) remain wholly literal. Consecutive `and` connectors stay in the malformed cardinal span instead of converting its valid fragments. Spoken punctuation owns interleaved horizontal spaces/tabs and adjacent marks on both sides, without crossing line breaks or changing punctuation that has no spoken cue.

Valid attached percent escapes also remain literal (`twenty-five%20items`); terminal percentage signs still format normally. Article-led minor cardinals use the same contextual grammar as major amounts, so unsupported `and a hundred cents` clauses are preserved completely.

Written numeric date forms such as `2026-10-04` remain literal. Ordinary multiword cardinals still convert beside calendar words (`October twenty five, 2026` → `October 25, 2026`); that changes the number's spelling without inferring or rearranging date layout. The formatter does not interpret month/day order or expand ordinals into date fields. Monetary lookback supports a cue within four tokens through recognized short copulas, modifiers or payment pronouns (`price is about five pounds`, `owe me five pounds`), stopping at punctuation, line breaks and other content words to avoid treating a later weight as a price. Smart opening single quotes permit quantity conversion, and typographic word-internal apostrophes retain the mixed-case name safeguard (`d’Artagnan`).

Converting standalone multiword numbers still carries the explicitly chosen tradeoff: a lowercase name or idiom that is also a valid English number phrase can be converted. The formatter cannot recover a name that a recognizer already emitted as digits. Calendar/date layout, telephone/PIN formatting, ordinals, fractions, scientific notation and non-English grammars are outside this pass. Disable spoken formatting for literal/code-oriented dictation; profile overrides and later personalization remain available.

## Regression evidence

Domain tests cover command punctuation ownership, sentence/self-correction casing, literal punctuation preservation, formatting opt-out, personalization order and idempotent processing. Quantity tests cover numeric grammar, decimals, exact currency rendering, signs, percentages, weight/currency ambiguity and preservation boundaries. Synthetic fixture replays go through the installed recognizers and this same cleanup stage. See [the cleanup benchmark evidence](../benchmarks/transcription/CLEANUP-RESULTS.md) for measured results and hardware limitations.

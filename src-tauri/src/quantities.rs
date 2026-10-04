//! Conservative English quantity formatting. No floating-point arithmetic,
//! rewriting service, language guessing, or changes to ordinary single words.

struct Token<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut result = vec![];
    let mut cursor = 0;
    while cursor < text.len() {
        let start = cursor;
        let c = text[cursor..].chars().next().unwrap();
        cursor += c.len_utf8();
        if c.is_whitespace() {
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            while let Some(next) = text[cursor..].chars().next() {
                let decimal_separator = c.is_ascii_digit()
                    && matches!(next, '.' | ',')
                    && text[cursor + 1..].starts_with(|c: char| c.is_ascii_digit());
                if !next.is_alphanumeric() && next != '_' && !decimal_separator {
                    break;
                }
                cursor += next.len_utf8();
            }
        }
        result.push(Token {
            start,
            end: cursor,
            text: &text[start..cursor],
        });
    }
    result
}

fn small(word: &str) -> Option<u64> {
    match word {
        "zero" | "oh" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        "sixty" => Some(60),
        "seventy" => Some(70),
        "eighty" => Some(80),
        "ninety" => Some(90),
        _ => None,
    }
}

fn scale(word: &str) -> Option<u64> {
    match word {
        "thousand" => Some(1_000),
        "million" => Some(1_000_000),
        "billion" => Some(1_000_000_000),
        "trillion" => Some(1_000_000_000_000),
        _ => None,
    }
}

fn number_word(word: &str) -> bool {
    small(word).is_some() || scale(word).is_some() || matches!(word, "hundred" | "point")
}

fn number_component(word: &str) -> bool {
    number_word(word) || word.starts_with(|c: char| c.is_ascii_digit())
}

fn atom(word: &str) -> Option<u64> {
    if word == "oh" {
        return None;
    }
    small(word).or_else(|| word.parse::<u64>().ok().filter(|value| *value < 1000))
}

fn group(words: &[&str]) -> Option<(u64, usize)> {
    let first = if words.first() == Some(&"a")
        && words
            .get(1)
            .is_some_and(|word| *word == "hundred" || scale(word).is_some())
    {
        1
    } else {
        atom(words.first()?)?
    };
    // "oh" is supported only in decimal digit sequences.
    if words[0] == "oh" {
        return None;
    }
    let (mut value, mut cursor) = if (1..=9).contains(&first) && words.get(1) == Some(&"hundred") {
        (first * 100, 2)
    } else {
        (0, 0)
    };
    if cursor != 0 && words.get(cursor) == Some(&"and") {
        cursor += 1;
        if cursor == words.len() {
            return None;
        }
    }
    let tail = if cursor == 0 {
        Some(first)
    } else {
        words
            .get(cursor)
            .and_then(|word| atom(word))
            .filter(|value| *value < 100)
    };
    if let Some(next) = tail {
        value += next;
        cursor += 1;
        if words
            .get(cursor - 1)
            .is_some_and(|word| small(word).is_some_and(|value| value >= 20))
        {
            if let Some(unit) = words
                .get(cursor)
                .and_then(|word| atom(word))
                .filter(|unit| (1..=9).contains(unit))
            {
                value += unit;
                cursor += 1;
            }
        }
    }
    (cursor != 0).then_some((value, cursor))
}

fn cardinal(words: &[&str]) -> Option<u64> {
    if words.is_empty() {
        return None;
    }
    let mut total = 0_u64;
    let mut cursor = 0;
    let mut previous_scale = u64::MAX;
    while cursor < words.len() {
        let (value, length) = group(&words[cursor..])?;
        cursor += length;
        if let Some(magnitude) = words.get(cursor).and_then(|word| scale(word)) {
            if value == 0 || magnitude >= previous_scale {
                return None;
            }
            total = total.checked_add(value.checked_mul(magnitude)?)?;
            previous_scale = magnitude;
            cursor += 1;
            if words.get(cursor) == Some(&"and") {
                cursor += 1;
                if cursor == words.len() {
                    return None;
                }
            }
        } else {
            if cursor != words.len() {
                return None;
            }
            total = total.checked_add(value)?;
        }
    }
    Some(total)
}

fn spoken(words: &[&str]) -> Option<String> {
    let point = words.iter().position(|word| *word == "point");
    if let Some(point) = point {
        let whole = if point == 0 {
            "0".to_owned()
        } else if point == 1 && words[0].starts_with(|c: char| c.is_ascii_digit()) {
            literal(words[0]).filter(|value| !value.contains('.'))?
        } else {
            cardinal(&words[..point])?.to_string()
        };
        let fraction = &words[point + 1..];
        if fraction.is_empty() || fraction.len() > 12 {
            return None;
        }
        let mut digits = String::new();
        for word in fraction {
            if word.bytes().all(|c| c.is_ascii_digit()) {
                digits.push_str(word);
            } else {
                let digit = small(word).filter(|digit| *digit < 10)?;
                digits.push(char::from(b'0' + digit as u8));
            }
        }
        if digits.len() > 12 {
            return None;
        }
        Some(format!("{whole}.{digits}"))
    } else if words.len() == 1 && words[0].starts_with(|c: char| c.is_ascii_digit()) {
        literal(words[0])
    } else {
        cardinal(words).map(|value| value.to_string())
    }
}

fn literal(text: &str) -> Option<String> {
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text, None), |(a, b)| (a, Some(b)));
    if whole.contains(',') {
        let groups: Vec<_> = whole.split(',').collect();
        if groups[0].is_empty() || groups[0].len() > 3 || groups[1..].iter().any(|g| g.len() != 3) {
            return None;
        }
    }
    let whole = whole.replace(',', "");
    if whole.is_empty() || whole.len() > 18 || !whole.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || fraction.len() > 12
            || !fraction.bytes().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        Some(format!("{whole}.{fraction}"))
    } else {
        Some(whole.to_owned())
    }
}

fn currency(word: &str) -> Option<&'static str> {
    match word {
        "dollar" | "dollars" => Some("$"),
        "euro" | "euros" => Some("€"),
        "pound" | "pounds" => Some("£"),
        _ => None,
    }
}

fn currency_annotation(word: &str) -> Option<&'static str> {
    match word {
        "$" => Some("$"),
        "€" => Some("€"),
        "£" => Some("£"),
        _ => currency(word),
    }
}

fn monetary_cue(word: &str) -> bool {
    matches!(
        word,
        "cost"
            | "costs"
            | "pay"
            | "paid"
            | "price"
            | "charge"
            | "charged"
            | "spend"
            | "spent"
            | "budget"
            | "owe"
            | "owes"
    )
}

fn quantity_unit(word: &str) -> bool {
    matches!(
        word,
        "box"
            | "boxes"
            | "item"
            | "items"
            | "file"
            | "files"
            | "user"
            | "users"
            | "person"
            | "people"
            | "page"
            | "pages"
            | "word"
            | "words"
            | "second"
            | "seconds"
            | "minute"
            | "minutes"
            | "hour"
            | "hours"
            | "day"
            | "days"
            | "week"
            | "weeks"
            | "month"
            | "months"
            | "year"
            | "years"
            | "millisecond"
            | "milliseconds"
            | "ms"
            | "meter"
            | "meters"
            | "metre"
            | "metres"
            | "centimeter"
            | "centimeters"
            | "centimetre"
            | "centimetres"
            | "cm"
            | "km"
            | "kilometer"
            | "kilometers"
            | "kilometre"
            | "kilometres"
            | "kilogram"
            | "kilograms"
            | "kg"
            | "gram"
            | "grams"
            | "g"
            | "liter"
            | "liters"
            | "litre"
            | "litres"
            | "ml"
            | "milliliter"
            | "milliliters"
            | "degree"
            | "degrees"
            | "byte"
            | "bytes"
            | "kb"
            | "mb"
            | "gb"
            | "tb"
            | "inch"
            | "inches"
            | "foot"
            | "feet"
            | "mile"
            | "miles"
            | "ounce"
            | "ounces"
            | "pound"
            | "pounds"
    )
}

fn gap(text: &str, left: &Token<'_>, right: &Token<'_>) -> bool {
    text[left.end..right.start]
        .chars()
        .all(|c| matches!(c, ' ' | '\t'))
}

fn money_amount(amount: &str) -> String {
    let Some((whole, fraction)) = amount.split_once('.') else {
        return amount.to_owned();
    };
    let fraction = fraction.trim_end_matches('0');
    match fraction.len() {
        0 => whole.to_owned(),
        1 => format!("{whole}.{fraction}0"),
        _ => format!("{whole}.{fraction}"),
    }
}

struct MinorAmount {
    fraction: String,
    end: usize,
    pence: bool,
}

fn minor_amount(
    text: &str,
    tokens: &[Token<'_>],
    lower: &[String],
    start: usize,
) -> Option<MinorAmount> {
    if lower.get(start)?.as_str() != "and" {
        return None;
    }
    let mut end = start + 1;
    while end < tokens.len()
        && gap(text, &tokens[end - 1], &tokens[end])
        && (number_component(&lower[end])
            || lower[end] == "and"
            || (lower[end] == "-"
                && tokens[end - 1].end == tokens[end].start
                && tokens
                    .get(end + 1)
                    .is_some_and(|next| next.start == tokens[end].end))
            || (end == start + 1 && literal(tokens[end].text).is_some()))
    {
        end += 1;
    }
    let unit = lower.get(end)?.as_str();
    if !matches!(unit, "cent" | "cents" | "penny" | "pence")
        || !gap(text, &tokens[end - 1], &tokens[end])
    {
        return None;
    }
    let minor = if end == start + 2
        && tokens[start + 1]
            .text
            .starts_with(|c: char| c.is_ascii_digit())
    {
        tokens[start + 1].text.parse::<u64>().ok()?
    } else {
        let words: Vec<_> = lower[start + 1..end]
            .iter()
            .filter(|word| word.as_str() != "-")
            .map(String::as_str)
            .collect();
        cardinal(&words)?
    };
    if minor >= 100 {
        return None;
    }
    Some(MinorAmount {
        fraction: format!("{minor:02}"),
        end: end + 1,
        pence: matches!(unit, "penny" | "pence"),
    })
}

/// Convert quantities before personalization, only when spoken formatting is on.
/// Standalone multiword cardinal/decimal phrases are intentionally canonicalized.
/// Single words require known units; titles and identifier-like sequences survive.
pub(crate) fn normalize(text: &str) -> String {
    let tokens = tokens(text);
    let lower: Vec<_> = tokens
        .iter()
        .map(|token| token.text.to_ascii_lowercase())
        .collect();
    let mut output = String::with_capacity(text.len());
    let mut copied = 0;
    let mut i = 0;
    while i < tokens.len() {
        let start = i;
        let mut negative = false;
        if matches!(lower[i].as_str(), "minus" | "negative") {
            negative = true;
            i += 1;
        } else if lower[i] == "-"
            && tokens
                .get(i + 1)
                .is_some_and(|next| next.start == tokens[i].end)
        {
            negative = true;
            i += 1;
        }
        let mut symbol = None;
        if i < tokens.len() && matches!(tokens[i].text, "$" | "€" | "£") {
            symbol = Some(tokens[i].text);
            i += 1;
        }
        if i == tokens.len() {
            break;
        }
        if i > start && (start..i).any(|j| !gap(text, &tokens[j], &tokens[j + 1])) {
            i = start + 1;
            continue;
        }
        if symbol.is_some()
            && tokens[i].text == "-"
            && tokens
                .get(i + 1)
                .is_some_and(|next| next.start == tokens[i].end)
        {
            negative = true;
            i += 1;
        }
        let number_start = i;
        let amount = if number_component(&lower[i])
            || (lower[i] == "a"
                && lower
                    .get(i + 1)
                    .is_some_and(|word| word == "hundred" || scale(word).is_some()))
        {
            i += 1;
            while i < tokens.len() {
                if tokens[i].text == "-"
                    && tokens[i - 1].end == tokens[i].start
                    && tokens.get(i + 1).is_some_and(|next| {
                        next.start == tokens[i].end
                            && number_component(&next.text.to_ascii_lowercase())
                    })
                {
                    i += 1;
                    continue;
                }
                if !gap(text, &tokens[i - 1], &tokens[i]) {
                    break;
                }
                // A symbol already establishes the major currency unit. Keep
                // an explicit minor-unit clause out of the cardinal grammar.
                if lower[i] == "and"
                    && symbol.is_some()
                    && minor_amount(text, &tokens, &lower, i).is_some()
                {
                    break;
                }
                let is_connector = lower[i] == "and"
                    && tokens.get(i + 1).is_some_and(|_| {
                        number_component(&lower[i + 1]) && gap(text, &tokens[i], &tokens[i + 1])
                    });
                if !number_component(&lower[i]) && !is_connector {
                    break;
                }
                i += 1;
            }
            let words: Vec<_> = lower[number_start..i]
                .iter()
                .filter(|w| w.as_str() != "-")
                .map(String::as_str)
                .collect();
            spoken(&words)
        } else {
            i = start + 1;
            continue;
        };
        let number_end = i;
        let next = lower.get(i).map(String::as_str);
        let unit_gap = tokens
            .get(i)
            .is_some_and(|next| gap(text, &tokens[number_end - 1], next));
        let sterling = next.is_some_and(|word| matches!(word, "pound" | "pounds"))
            && lower
                .get(i + 1)
                .is_some_and(|word| word == "sterling" && gap(text, &tokens[i], &tokens[i + 1]));
        // "Pounds" alone is also a weight. Currency requires an explicit symbol,
        // sterling qualifier, or a nearby monetary verb/noun in the same clause.
        let money_cue = (0..start)
            .rev()
            .take(4)
            .take_while(|&index| {
                gap(text, &tokens[index], &tokens[index + 1])
                    && (monetary_cue(&lower[index])
                        || matches!(
                            lower[index].as_str(),
                            "is" | "are"
                                | "was"
                                | "were"
                                | "be"
                                | "been"
                                | "will"
                                | "has"
                                | "have"
                                | "about"
                                | "roughly"
                                | "approximately"
                                | "around"
                                | "just"
                                | "only"
                                | "exactly"
                                | "me"
                                | "him"
                                | "her"
                                | "them"
                                | "us"
                                | "you"
                                | "it"
                                | "at"
                                | "of"
                        ))
            })
            .any(|index| monetary_cue(&lower[index]));
        let named_currency = next.and_then(currency).filter(|_| unit_gap);
        let money = named_currency
            .filter(|currency| *currency != "£" || symbol == Some("£") || sterling || money_cue);
        let annotation_end = i + 1 + usize::from(sterling);
        let conflicting_suffix = named_currency.is_some_and(|first| {
            lower.get(annotation_end).is_some_and(|word| {
                currency_annotation(word).is_some_and(|second| first != second)
                    && gap(text, &tokens[annotation_end - 1], &tokens[annotation_end])
            })
        });
        let conflicting_currency = symbol
            .zip(named_currency)
            .is_some_and(|(prefix, suffix)| prefix != suffix)
            || conflicting_suffix
            || (unit_gap
                && symbol
                    .zip(next.and_then(currency_annotation))
                    .is_some_and(|(first, second)| first != second));
        if symbol.is_none() {
            symbol = money;
        }
        // Parse the complete minor clause before changing its major amount.
        // A conflicting unit/annotation must preserve the whole source span,
        // including multiword minor quantities that might otherwise convert later.
        let minor_start = if money.is_some() { annotation_end } else { i };
        let minor = symbol.and_then(|_| {
            tokens
                .get(minor_start)
                .filter(|next| gap(text, &tokens[minor_start - 1], next))
                .and_then(|_| minor_amount(text, &tokens, &lower, minor_start))
        });
        if let (Some(symbol), Some(minor)) = (symbol, minor.as_ref()) {
            let conflicting_minor = minor.pence != (symbol == "£")
                || tokens.get(minor.end).is_some_and(|next| {
                    currency_annotation(&lower[minor.end])
                        .is_some_and(|currency| currency != symbol)
                        && gap(text, &tokens[minor.end - 1], next)
                });
            if conflicting_minor {
                i = minor.end;
                continue;
            }
        }
        let per_cent = next == Some("per")
            && lower
                .get(i + 1)
                .is_some_and(|word| word == "cent" && gap(text, &tokens[i], &tokens[i + 1]));
        let percent = (next.is_some_and(|w| matches!(w, "%" | "percent")) || per_cent) && unit_gap;
        let unit = next.is_some_and(quantity_unit) && unit_gap;
        let uppercase_unit_abbreviation = unit
            && next.is_some_and(|word| word.len() <= 2)
            && tokens[number_end]
                .text
                .bytes()
                .all(|c| c.is_ascii_uppercase());
        let multiword = number_end - number_start > 1;
        // An initial title/name such as "Twenty One Pilots" has no quantity
        // context. Do not guess that it denotes a number.
        let title = symbol.is_none()
            && !percent
            && !uppercase_unit_abbreviation
            && tokens
                .get(number_end)
                .is_some_and(|next| next.text.chars().next().is_some_and(char::is_uppercase));
        let attached = text[..tokens[start].start]
            .chars()
            .next_back()
            .is_some_and(|c| {
                !c.is_whitespace() && !matches!(c, '(' | '[' | '{' | '"' | '\'' | '“' | '‘')
            });
        let identifier_tail = tokens.get(number_end).is_some_and(|next| {
            next.start == tokens[number_end - 1].end
                && (matches!(next.text, "/" | "\\" | ":" | "_" | "-" | "@")
                    || (next.text == "."
                        && tokens.get(number_end + 1).is_some_and(|tail| {
                            tail.start == next.end && tail.text.starts_with(char::is_alphanumeric)
                        })))
        });
        let spaced_operator = start > 0
            && tokens[start - 1].text == "-"
            && tokens[start - 1].end < tokens[start].start
            && gap(text, &tokens[start - 1], &tokens[start]);
        let dangling_and = next == Some("and")
            && tokens
                .get(i + 1)
                .is_none_or(|token| !token.text.chars().any(char::is_alphanumeric));
        if let Some(amount) = amount.filter(|_| {
            !attached
                && !spaced_operator
                && !identifier_tail
                && !title
                && !dangling_and
                && !conflicting_currency
                && !(symbol.is_some() && percent)
                && (symbol.is_some()
                    || percent
                    || unit
                    || (multiword
                        && lower[number_start] != "a"
                        && lower[number_start..number_end]
                            .iter()
                            .any(|word| number_word(word))))
        }) {
            let mut amount = amount;
            // All-zero fractions denote an integer major amount; normalize
            // before deciding whether a minor clause can supply its fraction.
            if symbol.is_some() {
                amount = money_amount(&amount);
            }
            if money.is_some() {
                i += 1;
                if sterling {
                    i += 1;
                }
            }
            if !amount.contains('.') {
                if let Some(minor) = minor {
                    amount.push('.');
                    amount.push_str(&minor.fraction);
                    i = minor.end;
                }
            }
            if percent {
                i += 1;
                if per_cent {
                    i += 1;
                }
            }
            output.push_str(&text[copied..tokens[start].start]);
            if negative {
                output.push('-');
            }
            if let Some(symbol) = symbol {
                output.push_str(symbol);
            }
            output.push_str(&if symbol.is_some() {
                money_amount(&amount)
            } else {
                amount
            });
            if percent {
                output.push('%');
            }
            copied = tokens[i - 1].end;
        }
    }
    output.push_str(&text[copied..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_currency_symbols_remain_literal() {
        for source in [
            "five dollars £",
            "five euros $",
            "$five €",
            "one dollar and five cents £",
        ] {
            assert_eq!(normalize(source), source, "{source}");
        }
        assert_eq!(normalize("five dollars, £ later"), "$5, £ later");
        assert_eq!(normalize("five dollars\n£ later"), "$5\n£ later");
    }

    #[test]
    fn zero_fraction_major_amounts_combine_minor_units() {
        for (source, expected) in [
            ("$1.00 and five cents", "$1.05"),
            ("1.00 dollars and five cents", "$1.05"),
            ("€2.000 and 50 cents", "€2.50"),
            ("£0.00 and five pence", "£0.05"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
    }

    #[test]
    fn windows_path_components_remain_literal() {
        for source in [
            r"twenty five\report",
            r"C:\twenty five\report",
            r"twenty-five\report",
        ] {
            assert_eq!(normalize(source), source, "{source}");
        }
    }

    #[test]
    fn singular_minor_currency_units_are_supported() {
        for (source, expected) in [
            ("one dollar and one cent", "$1.01"),
            ("€1 and one cent", "€1.01"),
            ("£1 and one penny", "£1.01"),
            ("one pound sterling and 1 penny", "£1.01"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
        for source in ["one dollar and one penny", "£1 and one cent"] {
            assert_eq!(normalize(source), source, "{source}");
        }
    }

    #[test]
    fn monetary_cues_allow_short_copulas_but_stop_at_content_boundaries() {
        for (source, expected) in [
            ("price is about five pounds", "price is about £5"),
            ("cost was roughly five pounds", "cost was roughly £5"),
            ("costs just five pounds", "costs just £5"),
            ("owe me five pounds", "owe me £5"),
            ("price will be about five pounds", "price will be about £5"),
            (
                "price is high but weighs five pounds",
                "price is high but weighs 5 pounds",
            ),
            (
                "price is high, about five pounds",
                "price is high, about 5 pounds",
            ),
            ("price is\nabout five pounds", "price is\nabout 5 pounds"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
    }

    #[test]
    fn quantities_inside_smart_single_quotes_format() {
        assert_eq!(normalize("‘five boxes’"), "‘5 boxes’");
        assert_eq!(normalize("‘twenty five’"), "‘25’");
        assert_eq!(normalize("‘one more thing’"), "‘one more thing’");
    }

    #[test]
    fn cardinal_date_words_do_not_imply_date_layout_inference() {
        assert_eq!(normalize("October twenty five, 2026"), "October 25, 2026");
        assert_eq!(normalize("twenty five of October"), "25 of October");
        assert_eq!(normalize("2026-10-04"), "2026-10-04");
    }

    #[test]
    fn conflicting_minor_currency_clauses_remain_wholly_literal() {
        for source in [
            "one dollar and five pence",
            "one dollar and twenty five pence",
            "one pound sterling and five cents",
            "$1 and five cents euros",
            "one dollar and five cents euros",
            "two euros and ninety nine cents dollars",
            "£1 and five pence dollars",
        ] {
            assert_eq!(normalize(source), source, "{source}");
        }
        assert_eq!(
            normalize("one dollar and five cents, euros later"),
            "$1.05, euros later"
        );
        assert_eq!(
            normalize("one dollar and five cents\neuros later"),
            "$1.05\neuros later"
        );
    }

    #[test]
    fn uppercase_unit_abbreviations_remain_quantity_context() {
        for (source, expected) in [
            ("five GB", "5 GB"),
            ("twelve MB", "12 MB"),
            ("twenty five KB", "25 KB"),
            ("one TB", "1 TB"),
            ("five KG", "5 KG"),
            ("Five Seconds of Summer", "Five Seconds of Summer"),
            ("Seven Pounds", "Seven Pounds"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
    }

    #[test]
    fn symbol_prefixed_major_amounts_combine_minor_units() {
        for (source, expected) in [
            ("$1 and five cents", "$1.05"),
            ("€2 and ninety nine cents", "€2.99"),
            ("£1 and five pence", "£1.05"),
            ("$-1 and five cents", "-$1.05"),
            ("$1 dollars and five cents", "$1.05"),
            ("$one hundred and five dollars", "$105"),
            ("$1 and five pence", "$1 and five pence"),
            ("£1 and five cents", "£1 and five cents"),
            ("$1\nand five cents", "$1\nand five cents"),
        ] {
            let result = normalize(source);
            assert_eq!(result, expected, "{source}");
            assert_eq!(normalize(&result), result, "not idempotent: {source}");
        }
    }

    #[test]
    fn capitalized_unit_titles_remain_names() {
        for (source, expected) in [
            ("Five Seconds of Summer", "Five Seconds of Summer"),
            ("Seven Pounds", "Seven Pounds"),
            (
                "One Hundred Years of Solitude",
                "One Hundred Years of Solitude",
            ),
            ("wait five seconds", "wait 5 seconds"),
            ("Five seconds remain", "5 seconds remain"),
            ("weighs seven pounds", "weighs 7 pounds"),
            ("one hundred years", "100 years"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
    }

    #[test]
    fn conflicting_trailing_currency_names_remain_literal() {
        for source in [
            "five dollars euros",
            "five euros dollars",
            "five pounds sterling dollars",
            "$5 dollars euros",
            "$five pounds",
            "five pounds dollars",
        ] {
            assert_eq!(normalize(source), source, "{source}");
        }
        assert_eq!(normalize("five dollars, euros later"), "$5, euros later");
        assert_eq!(normalize("five dollars\neuros later"), "$5\neuros later");
    }

    #[test]
    fn review_preservation_boundaries() {
        for (source, expected) in [
            ("twenty - five boxes", "twenty - five boxes"),
            ("forty - two percent", "forty - two percent"),
            ("twenty- five boxes", "twenty- five boxes"),
            ("twenty -five boxes", "twenty -5 boxes"),
            ("twenty-five boxes", "25 boxes"),
            ("one dollar and twenty-five cents", "$1.25"),
            (
                "one dollar and twenty - five cents",
                "$1 and twenty - five cents",
            ),
            ("shipping costs\nfive pounds", "shipping costs\n5 pounds"),
            (
                "shipping costs\r\nabout five pounds",
                "shipping costs\r\nabout 5 pounds",
            ),
            (
                "shipping costs\n\nfive pounds",
                "shipping costs\n\n5 pounds",
            ),
            (
                "shipping costs about five pounds",
                "shipping costs about £5",
            ),
            ("twenty-five.com", "twenty-five.com"),
            ("version twenty five.1", "version twenty five.1"),
            ("twenty five.", "25."),
            ("twenty five. next", "25. next"),
        ] {
            let result = normalize(source);
            assert_eq!(result, expected, "{source}");
            assert_eq!(normalize(&result), result, "not idempotent: {source}");
        }
    }

    #[test]
    fn cardinal_grammar_and_exact_decimals() {
        for (source, expected) in [
            ("twenty five", "25"),
            ("forty-five", "45"),
            ("one hundred and five", "105"),
            ("one thousand and five", "1005"),
            ("two million three hundred thousand seven", "2300007"),
            (
                "one trillion two billion three million four thousand five",
                "1002003004005",
            ),
            ("twelve point zero five", "12.05"),
            ("zero point oh five", "0.05"),
            ("point five", "0.5"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
        for source in [
            "one two three",
            "twenty four seven",
            "one million million",
            "one point twenty five",
            "one hundred and",
        ] {
            assert_eq!(normalize(source), source);
        }
    }

    #[test]
    fn quantities_currency_percent_and_negative_amounts() {
        for (source, expected) in [
            ("five boxes and twelve users", "5 boxes and 12 users"),
            ("twelve point five kilograms", "12.5 kilograms"),
            (
                "The order contains twelve boxes and costs forty five dollars.",
                "The order contains 12 boxes and costs $45.",
            ),
            (
                "45 dollars, forty five euros, and costs five pounds.",
                "$45, €45, and costs £5.",
            ),
            ("minus twelve point five degrees", "-12.5 degrees"),
            ("negative forty five dollars", "-$45"),
            ("-12.50 dollars", "-$12.50"),
            ("$1,234.50", "$1234.50"),
            ("$-12.50", "-$12.50"),
            ("$45.00", "$45"),
            ("$0.0050", "$0.005"),
            ("twelve point five dollars", "$12.50"),
            ("one dollar and five cents", "$1.05"),
            ("two euros and ninety nine cents", "€2.99"),
            ("45 dollars and 50 cents", "$45.50"),
            ("one pound sterling and five pence", "£1.05"),
            ("twelve point five per cent", "12.5%"),
            ("five pounds sterling", "£5"),
            ("weighs five pounds", "weighs 5 pounds"),
            ("one point five percent and 12.5%", "1.5% and 12.5%"),
            ("(five boxes)", "(5 boxes)"),
        ] {
            assert_eq!(normalize(source), expected, "{source}");
        }
    }

    #[test]
    fn ambiguous_prose_names_dates_codes_and_non_english_survive() {
        for source in [
            "one more thing",
            "one of a kind",
            "one in a million",
            "a hundred",
            "a hundred reasons",
            "take five",
            "twenty four seven",
            "Twenty One Pilots",
            "Seven Oaks",
            "Version v0.5.5",
            "2026-10-04",
            "PIN one two three four",
            "https://example.com/twenty-five",
            "id_twenty_five",
            "12,34 dollars",
            "12.5.5 kilograms",
            "πέντε κιλά",
            "一百美元",
            "two point five million dollars",
            "twenty\nfive",
            "$\n45",
            "$45 euros",
            "$45 percent",
        ] {
            assert_eq!(normalize(source), source, "{source}");
        }
    }

    #[test]
    fn mixed_recognizer_numbers_are_parsed_as_one_phrase() {
        for (raw, expected) in [
            ("12 point five kilograms", "12.5 kilograms"),
            ("twelve point 05 kilograms", "12.05 kilograms"),
            ("$12 point five", "$12.50"),
            ("one hundred 5 boxes", "105 boxes"),
            ("12 million dollars", "$12000000"),
            ("a hundred boxes", "100 boxes"),
            ("a million dollars", "$1000000"),
            ("one dollar and forty 5 cents", "$1.45"),
            ("a box and one more thing", "a box and one more thing"),
            ("one hundred oh", "one hundred oh"),
            ("12.5 point five kilograms", "12.5 point five kilograms"),
            ("PIN 123 456 7890", "PIN 123 456 7890"),
        ] {
            let result = normalize(raw);
            assert_eq!(result, expected, "{raw}");
            assert_eq!(normalize(&result), result, "not idempotent: {raw}");
        }
    }
}

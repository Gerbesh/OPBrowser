//! Original character-reference consumer: common named subset + all numeric
//! references. Rules: https://html.spec.whatwg.org/multipage/parsing.html#character-reference-state

pub(super) fn consume(input: &[char], attribute: bool) -> Option<(char, usize)> {
    if input.first() == Some(&'#') {
        return numeric(input);
    }
    NAMED
        .iter()
        .filter_map(|(name, character, legacy)| {
            if !input.iter().copied().zip(name.chars()).all(|(a, b)| a == b)
                || input.len() < name.len()
            {
                return None;
            }
            let length = name.len();
            let semicolon = input.get(length) == Some(&';');
            if !semicolon
                && (!legacy
                    || (attribute
                        && input
                            .get(length)
                            .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '=')))
            {
                return None;
            }
            Some((*character, length + usize::from(semicolon)))
        })
        .max_by_key(|(_, length)| *length)
}

fn numeric(input: &[char]) -> Option<(char, usize)> {
    let hexadecimal = matches!(input.get(1), Some('x' | 'X'));
    let radix = if hexadecimal { 16 } else { 10 };
    let start = if hexadecimal { 2 } else { 1 };
    let mut end = start;
    let mut value = 0u32;
    while let Some(digit) = input.get(end).and_then(|ch| ch.to_digit(radix)) {
        value = value.saturating_mul(radix).saturating_add(digit);
        end += 1;
    }
    if end == start {
        return None;
    }
    if input.get(end) == Some(&';') {
        end += 1;
    }
    let value = match value {
        0 => 0xfffd,
        0x80..=0x9f => u32::from(C1_REPLACEMENTS[(value - 0x80) as usize]),
        other => other,
    };
    Some((char::from_u32(value).unwrap_or('\u{fffd}'), end))
}

// Names are case-sensitive. `true` identifies legacy names accepted without ';'.
// Source: https://html.spec.whatwg.org/entities.json (subset only).
const NAMED: &[(&str, char, bool)] = &[
    ("amp", '&', true),
    ("AMP", '&', true),
    ("lt", '<', true),
    ("LT", '<', true),
    ("gt", '>', true),
    ("GT", '>', true),
    ("quot", '"', true),
    ("QUOT", '"', true),
    ("apos", '\'', false),
    ("nbsp", '\u{a0}', true),
    ("copy", '©', true),
    ("COPY", '©', true),
    ("reg", '®', true),
    ("REG", '®', true),
    ("euro", '€', false),
    ("trade", '™', false),
    ("ndash", '–', false),
    ("mdash", '—', false),
    ("hellip", '…', false),
    ("laquo", '«', true),
    ("raquo", '»', true),
    ("times", '×', true),
    ("divide", '÷', true),
    ("bull", '•', false),
];

// HTML's numeric-reference legacy C1 replacement table. Unmapped controls stay intact.
const C1_REPLACEMENTS: [u16; 32] = [
    0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
];

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(value: &str, attribute: bool) -> Option<(char, usize)> {
        consume(&value.chars().collect::<Vec<_>>(), attribute)
    }

    #[test]
    fn handles_named_reference_attribute_ambiguity() {
        assert_eq!(parse("amp;next", true), Some(('&', 4)));
        assert_eq!(parse("amp=next", true), None);
        assert_eq!(parse("ampx", true), None);
        assert_eq!(parse("ampx", false), Some(('&', 3)));
        assert_eq!(parse("apos", false), None);
        assert_eq!(parse("unknown;", false), None);
        assert_eq!(parse("Amp;", false), None);
        assert_eq!(parse("a", false), None);
    }

    #[test]
    fn handles_numeric_recovery_and_unicode_scalars() {
        for (input, expected) in [
            ("#1040;", 'А'),
            ("#x1F600;", '😀'),
            ("#128;", '€'),
            ("#0;", '\u{fffd}'),
            ("#xD800;", '\u{fffd}'),
            ("#999999999999999999999999;", '\u{fffd}'),
        ] {
            assert_eq!(parse(input, false), Some((expected, input.len())));
        }
        assert_eq!(parse("#65x", true), Some(('A', 3)));
        assert_eq!(parse("#x;", false), None);
    }
}

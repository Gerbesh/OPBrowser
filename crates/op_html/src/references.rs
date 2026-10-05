//! Original consumer for the full named-reference table and numeric references.
//! Rules: https://html.spec.whatwg.org/multipage/parsing.html#character-reference-state

mod named;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Characters {
    first: char,
    second: Option<char>,
}

impl Characters {
    pub(super) fn single(first: char) -> Self {
        Self {
            first,
            second: None,
        }
    }

    fn named(value: &str) -> Self {
        let mut chars = value.chars();
        Self {
            first: chars.next().expect("validated nonempty named replacement"),
            second: chars.next(),
        }
    }

    pub(super) fn iter(self) -> impl Iterator<Item = char> {
        std::iter::once(self.first).chain(self.second)
    }
}

// Eight bytes per entry, with no pointers or runtime allocation: name/value byte
// offsets, name/value byte lengths, legacy spelling flag. Generated UTF-8 value
// offsets are scalar boundaries; names are sorted ASCII without their semicolon.
struct NamedEntry(u16, u16, u8, u8, bool);

impl NamedEntry {
    fn name(&self) -> &[u8] {
        let start = usize::from(self.0);
        &named::NAMES.as_bytes()[start..start + usize::from(self.2)]
    }

    fn value(&self) -> &str {
        let start = usize::from(self.1);
        &named::VALUES[start..start + usize::from(self.3)]
    }
}

pub(super) fn consume(input: &[char], attribute: bool) -> Option<(Characters, usize)> {
    if input.first() == Some(&'#') {
        return numeric(input);
    }

    // Narrow a sorted prefix range with two binary searches per ASCII character.
    // Remember a legacy match while seeking a longer semicolon-terminated name.
    // Work is bounded by the longest standard name, even on huge unknown input.
    let mut entries = named::ENTRIES;
    let mut legacy_match = None;
    for (index, ch) in input.iter().copied().take(named::MAX_NAME_LEN).enumerate() {
        if !ch.is_ascii_alphanumeric() {
            break;
        }
        let byte = ch as u8;
        let start = entries.partition_point(|e| e.name().get(index).copied().unwrap_or(0) < byte);
        let end = entries.partition_point(|e| e.name().get(index).copied().unwrap_or(0) <= byte);
        entries = &entries[start..end];
        let Some(entry) = entries.first() else {
            break;
        };
        let length = index + 1;
        if usize::from(entry.2) == length {
            if input.get(length) == Some(&';') {
                return Some((Characters::named(entry.value()), length + 1));
            }
            if entry.4 {
                legacy_match = Some((entry, length));
            }
        }
    }
    let (entry, length) = legacy_match?;
    if attribute
        && input
            .get(length)
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '=')
    {
        return None;
    }
    Some((Characters::named(entry.value()), length))
}

fn numeric(input: &[char]) -> Option<(Characters, usize)> {
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
    Some((
        Characters::single(char::from_u32(value).unwrap_or('\u{fffd}')),
        end,
    ))
}

// HTML's numeric-reference legacy C1 replacement table. Unmapped controls stay intact.
const C1_REPLACEMENTS: [u16; 32] = [
    0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
];

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(value: &str, attribute: bool) -> Option<(String, usize)> {
        consume(&value.chars().collect::<Vec<_>>(), attribute)
            .map(|(chars, consumed)| (chars.iter().collect(), consumed))
    }

    #[test]
    fn handles_named_reference_attribute_ambiguity() {
        assert_eq!(parse("amp;next", true), Some(("&".into(), 4)));
        assert_eq!(parse("amp=next", true), None);
        assert_eq!(parse("ampx", true), None);
        assert_eq!(parse("ampx", false), Some(("&".into(), 3)));
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
            assert_eq!(
                parse(input, false),
                Some((expected.to_string(), input.len()))
            );
        }
        assert_eq!(parse("#65x", true), Some(("A".into(), 3)));
        assert_eq!(parse("#x;", false), None);
    }

    #[test]
    fn keeps_longest_match_and_legacy_prefix_fallback() {
        for (input, expected, consumed) in [
            ("notin;", "∉", 6),
            ("notin", "¬", 3),
            ("notit;", "¬", 3),
            ("notinva;", "∉", 8),
            ("NotEqualTilde;", "\u{2242}\u{338}", 14),
            ("CounterClockwiseContourIntegral;", "∳", 32),
            ("sup;", "⊃", 4),
            ("sup1;", "¹", 5),
        ] {
            assert_eq!(
                parse(input, false),
                Some((expected.into(), consumed)),
                "{input}"
            );
        }
        assert_eq!(parse("notit;", true), None);
        assert_eq!(parse("notin", true), None);
        assert_eq!(parse("not=", true), None);
        assert_eq!(parse("not!", true), Some(("¬".into(), 3)));
        assert_eq!(parse("NotEqualTilde", false), None);
        assert_eq!(parse(&"z".repeat(100_000), false), None);
        assert_eq!(parse("αmp;", false), None);
    }

    #[test]
    fn validates_compact_table_bounds_order_and_scalar_results() {
        assert_eq!(std::mem::size_of::<NamedEntry>(), 8);
        assert_eq!(named::ENTRIES.len(), 2125);
        assert_eq!(named::ENTRIES.iter().filter(|e| e.4).count(), 106);
        for entries in named::ENTRIES.windows(2) {
            assert!(entries[0].name() < entries[1].name());
        }
        for entry in named::ENTRIES {
            assert!(entry.name().iter().all(u8::is_ascii_alphanumeric));
            assert!(entry.name().len() <= named::MAX_NAME_LEN);
            assert!((1..=2).contains(&entry.value().chars().count()));
        }
    }
}

//! Compact opaque CSS named colors; special keywords remain in computed-value parsing.

use crate::CssColor;

mod data;

struct Entry {
    offset: u16,
    length: u8,
    rgb: [u8; 3],
}

impl Entry {
    const fn new(offset: u16, length: u8, rgb: [u8; 3]) -> Self {
        Self {
            offset,
            length,
            rgb,
        }
    }

    fn name(&self) -> &'static str {
        let start = usize::from(self.offset);
        &data::NAMES[start..start + usize::from(self.length)]
    }
}

pub(crate) fn lookup(value: &str) -> Option<CssColor> {
    if value.is_empty() || value.len() > 20 {
        return None;
    }
    let index = data::ENTRIES
        .binary_search_by(|entry| {
            entry
                .name()
                .bytes()
                .cmp(value.bytes().map(|byte| byte.to_ascii_lowercase()))
        })
        .ok()?;
    let [red, green, blue] = data::ENTRIES[index].rgb;
    Some(CssColor {
        red,
        green,
        blue,
        alpha: 255,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pinned_name_and_case_variant_matches_numeric_source_without_table_growth() {
        let mut previous = "";
        let mut count = 0;
        for line in include_str!("../data/named-colors.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
        {
            let (name, hex) = line.split_once('\t').unwrap();
            assert!(name > previous);
            let color = u32::from_str_radix(hex, 16).unwrap();
            let expected = CssColor {
                red: (color >> 16) as u8,
                green: (color >> 8) as u8,
                blue: color as u8,
                alpha: 255,
            };
            assert_eq!(lookup(name), Some(expected));
            assert_eq!(lookup(&name.to_ascii_uppercase()), Some(expected));
            previous = name;
            count += 1;
        }
        assert_eq!(count, 148);
        assert_eq!(std::mem::size_of::<Entry>(), 6);
        assert_eq!(data::ENTRIES.len(), 148);
        assert!(data::NAMES.len() + std::mem::size_of_val(data::ENTRIES) < 2500);
    }

    #[test]
    fn aliases_and_unknown_or_non_ascii_names_are_unambiguous() {
        for (left, right) in [
            ("aqua", "cyan"),
            ("fuchsia", "magenta"),
            ("darkslategray", "darkslategrey"),
            ("lightgray", "lightgrey"),
        ] {
            assert_eq!(lookup(left), lookup(right));
        }
        assert_ne!(lookup("green"), lookup("lime"));
        for value in [
            "",
            "redd",
            " red",
            "red ",
            "transparent",
            "currentcolor",
            "синий",
            "ſteelblue",
            "lightgoldenrodyellowextra",
        ] {
            assert_eq!(lookup(value), None, "{value}");
        }
    }
}

//! First-party editable text control model. Indices are Unicode scalar positions,
//! never raw UTF-8 byte offsets; native typing must not split multi-byte text.
use op_dom::NodeId;

const MAX_TEXT_SCALARS: usize = 4096;

#[derive(Debug, Clone)]
pub(super) struct TextEditor {
    pub node: NodeId,
    pub caret: usize,
    pub anchor: usize,
}

#[derive(Debug, Clone)]
pub(super) enum Edit {
    Insert(String),
    Backspace,
    Delete,
    Left { shift: bool },
    Right { shift: bool },
    Home { shift: bool },
    End { shift: bool },
    SelectAll,
}

impl TextEditor {
    pub fn new(node: NodeId, value: &str) -> Self {
        let end = value.chars().count();
        Self {
            node,
            caret: end,
            anchor: end,
        }
    }

    /// The web-exposed selection uses UTF-16 code units; the native editor
    /// uses Unicode scalar indices. Positions in the middle of a surrogate
    /// pair are rounded back to the containing scalar to keep UTF-8 valid.
    pub fn set_utf16_selection(&mut self, value: &str, start: usize, end: usize, backward: bool) {
        let to_scalar = |utf16: usize| -> usize {
            let mut offset = 0;
            for (index, ch) in value.chars().enumerate() {
                if offset + ch.len_utf16() > utf16 {
                    return index;
                }
                offset += ch.len_utf16();
            }
            value.chars().count()
        };
        let start = to_scalar(start);
        let end = to_scalar(end).max(start);
        self.anchor = if backward { end } else { start };
        self.caret = if backward { start } else { end };
    }

    pub fn selection_utf16(&self, value: &str) -> (usize, usize, bool) {
        let offset =
            |scalar: usize| -> usize { value.chars().take(scalar).map(char::len_utf16).sum() };
        (
            offset(self.caret.min(self.anchor)),
            offset(self.caret.max(self.anchor)),
            self.caret < self.anchor,
        )
    }

    pub fn edit(&mut self, value: &str, edit: Edit) -> (String, bool) {
        let total = value.chars().count();
        self.caret = self.caret.min(total);
        self.anchor = self.anchor.min(total);
        let (start, end) = (self.caret.min(self.anchor), self.caret.max(self.anchor));
        let select = start != end;
        let mut next = value.to_owned();
        match edit {
            Edit::SelectAll => {
                self.anchor = 0;
                self.caret = total;
            }
            Edit::Left { shift } => {
                self.caret = if select && !shift {
                    start
                } else {
                    self.caret.saturating_sub(1)
                };
                if !shift {
                    self.anchor = self.caret;
                }
            }
            Edit::Right { shift } => {
                self.caret = if select && !shift {
                    end
                } else {
                    self.caret.saturating_add(1).min(total)
                };
                if !shift {
                    self.anchor = self.caret;
                }
            }
            Edit::Home { shift } => {
                self.caret = value
                    .chars()
                    .take(self.caret)
                    .collect::<String>()
                    .rfind('\n')
                    .map_or(0, |byte| value[..byte].chars().count() + 1);
                if !shift {
                    self.anchor = self.caret;
                }
            }
            Edit::End { shift } => {
                let byte = byte_at(value, self.caret);
                self.caret = value[byte..].find('\n').map_or(total, |next| {
                    self.caret + value[byte..byte + next].chars().count()
                });
                if !shift {
                    self.anchor = self.caret;
                }
            }
            Edit::Backspace | Edit::Delete | Edit::Insert(_) => {
                let (start, end) = if select {
                    (start, end)
                } else {
                    match edit {
                        Edit::Backspace => (self.caret.saturating_sub(1), self.caret),
                        Edit::Delete => (self.caret, (self.caret + 1).min(total)),
                        _ => (self.caret, self.caret),
                    }
                };
                let inserted = if let Edit::Insert(text) = &edit {
                    let allowed =
                        MAX_TEXT_SCALARS.saturating_sub(total.saturating_sub(end - start));
                    text.chars().take(allowed).collect::<String>()
                } else {
                    String::new()
                };
                if start < end || !inserted.is_empty() {
                    next.replace_range(byte_at(value, start)..byte_at(value, end), &inserted);
                    self.caret = start + inserted.chars().count();
                    self.anchor = self.caret;
                }
            }
        }
        let changed = next != value;
        (next, changed)
    }
}

/// Find the nearest Unicode scalar caret boundary under a native document click.
/// The current first-party field painter uses one unwrapped line per newline
/// and a fixed font-size + 4 line advance; use that same provisional geometry.
pub(super) fn caret_at_point(
    value: &str,
    x: i32,
    y: i32,
    font_size: i32,
    masked: bool,
    mut measure: impl FnMut(&str) -> i32,
) -> usize {
    let line_height = font_size.saturating_add(4).max(1);
    let wanted_row = (y.max(0) / line_height) as usize;
    let lines: Vec<&str> = value.split('\n').collect();
    let row = wanted_row.min(lines.len().saturating_sub(1));
    let start: usize = lines[..row]
        .iter()
        .map(|line| line.chars().count() + 1)
        .sum();
    let line = lines[row];
    let shown = if masked {
        "•".repeat(line.chars().count())
    } else {
        line.to_owned()
    };
    let count = shown.chars().count();
    if x <= 0 || count == 0 {
        return start;
    }
    // A bounded binary search avoids measuring every Unicode prefix of a
    // long field and never returns a position inside a UTF-8 scalar.
    let width_at = |index: usize, measure: &mut dyn FnMut(&str) -> i32| {
        measure(&shown.chars().take(index).collect::<String>()).max(0)
    };
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = low + (high - low) / 2;
        if width_at(mid, &mut measure) < x {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    let right = low;
    let left = right.saturating_sub(1);
    let right_distance = (i64::from(width_at(right, &mut measure)) - i64::from(x)).abs();
    let left_distance = (i64::from(width_at(left, &mut measure)) - i64::from(x)).abs();
    start
        + if right_distance <= left_distance {
            right
        } else {
            left
        }
}

fn byte_at(value: &str, index: usize) -> usize {
    value
        .char_indices()
        .nth(index)
        .map_or(value.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn click_to_caret_rounds_to_scalar_boundaries_on_each_line() {
        let width = |text: &str| text.chars().count() as i32 * 10;
        assert_eq!(caret_at_point("A😊B", -5, 0, 16, false, width), 0);
        assert_eq!(caret_at_point("A😊B", 11, 0, 16, false, width), 1);
        assert_eq!(caret_at_point("A😊B", 24, 0, 16, false, width), 2);
        assert_eq!(caret_at_point("A😊B", 90, 0, 16, false, width), 3);
        assert_eq!(caret_at_point("one\n猫😊", 11, 22, 16, false, width), 5);
        assert_eq!(caret_at_point("one\n猫😊", 90, 999, 16, false, width), 6);
        assert_eq!(caret_at_point("secret", 16, 0, 16, true, width), 2);
    }

    #[test]
    fn unicode_caret_delete_and_selection_never_split_utf8() {
        let mut e = TextEditor::new(op_dom::Document::new().root(), "Привет😊");
        let (s, _) = e.edit("Привет😊", Edit::Left { shift: false });
        let (s, ok) = e.edit(&s, Edit::Backspace);
        assert!(ok);
        assert_eq!(s, "Приве😊");
        e.edit(&s, Edit::SelectAll);
        let (s, _) = e.edit(&s, Edit::Insert("猫".into()));
        assert_eq!(s, "猫");
        e.set_utf16_selection("a😊b", 1, 3, true);
        assert_eq!(e.selection_utf16("a😊b"), (1, 3, true));
        let (after, _) = e.edit("a😊b", Edit::Insert("猫".into()));
        assert_eq!(after, "a猫b");
        e.set_utf16_selection("a😊b", 2, 2, false);
        assert_eq!(e.selection_utf16("a😊b"), (1, 1, false));
    }
}

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
    fn unicode_caret_delete_and_selection_never_split_utf8() {
        let mut e = TextEditor::new(op_dom::Document::new().root(), "Привет😊");
        let (s, _) = e.edit("Привет😊", Edit::Left { shift: false });
        let (s, ok) = e.edit(&s, Edit::Backspace);
        assert!(ok);
        assert_eq!(s, "Приве😊");
        e.edit(&s, Edit::SelectAll);
        let (s, _) = e.edit(&s, Edit::Insert("猫".into()));
        assert_eq!(s, "猫");
    }
}

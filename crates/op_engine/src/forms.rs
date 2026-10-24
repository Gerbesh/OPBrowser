//! First-party editable text control model. Indices are Unicode scalar positions,
//! never raw UTF-8 byte offsets; native typing must not split multi-byte text.
use op_dom::NodeId;

const MAX_TEXT_SCALARS: usize = 4096;
const MAX_UNDO_STEPS: usize = 32;

#[derive(Debug, Clone)]
struct EditorSnapshot {
    value: String,
    caret: usize,
    anchor: usize,
}

#[derive(Debug, Clone)]
pub(super) struct TextEditor {
    pub node: NodeId,
    pub caret: usize,
    pub anchor: usize,
    pub scroll_start: usize,
    observed_value: String,
    undo: Vec<EditorSnapshot>,
    redo: Vec<EditorSnapshot>,
}

#[derive(Debug, Clone)]
pub(super) enum Edit {
    Insert(String),
    DeleteSelection,
    Backspace,
    Delete,
    Left { shift: bool },
    Right { shift: bool },
    Home { shift: bool },
    End { shift: bool },
    SelectAll,
    Undo,
    Redo,
}

impl TextEditor {
    pub fn new(node: NodeId, value: &str) -> Self {
        let end = value.chars().count();
        Self {
            node,
            caret: end,
            anchor: end,
            scroll_start: 0,
            observed_value: value.to_owned(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Script-assigned values are not native edits. Discard stale native
    /// history rather than overwriting newer author JavaScript state on undo.
    pub fn observe_value(&mut self, value: &str) {
        if self.observed_value != value {
            self.undo.clear();
            self.redo.clear();
            self.observed_value = value.to_owned();
            self.scroll_start = 0;
            let count = value.chars().count();
            self.caret = self.caret.min(count);
            self.anchor = self.anchor.min(count);
        }
    }

    fn push_snapshot(stack: &mut Vec<EditorSnapshot>, snapshot: EditorSnapshot) {
        if stack.len() == MAX_UNDO_STEPS {
            stack.remove(0);
        }
        stack.push(snapshot);
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

    /// Extract a selected range without ever slicing a UTF-8 code point.
    pub fn selected_text(&self, value: &str) -> Option<String> {
        let total = value.chars().count();
        let start = self.caret.min(self.anchor).min(total);
        let end = self.caret.max(self.anchor).min(total);
        (start < end).then(|| value.chars().skip(start).take(end - start).collect())
    }

    pub fn edit(&mut self, value: &str, edit: Edit) -> (String, bool) {
        self.observe_value(value);
        let total = value.chars().count();
        self.caret = self.caret.min(total);
        self.anchor = self.anchor.min(total);
        if matches!(edit, Edit::Undo | Edit::Redo) {
            let current = EditorSnapshot {
                value: value.to_owned(),
                caret: self.caret,
                anchor: self.anchor,
            };
            let previous = if matches!(edit, Edit::Undo) {
                self.undo.pop().inspect(|_| {
                    Self::push_snapshot(&mut self.redo, current);
                })
            } else {
                self.redo.pop().inspect(|_| {
                    Self::push_snapshot(&mut self.undo, current);
                })
            };
            if let Some(previous) = previous {
                self.caret = previous.caret;
                self.anchor = previous.anchor;
                self.observed_value.clone_from(&previous.value);
                return (previous.value, true);
            }
            return (value.to_owned(), false);
        }
        let previous = EditorSnapshot {
            value: value.to_owned(),
            caret: self.caret,
            anchor: self.anchor,
        };
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
            Edit::Backspace | Edit::Delete | Edit::DeleteSelection | Edit::Insert(_) => {
                let (start, end) = if select {
                    (start, end)
                } else {
                    match edit {
                        Edit::Backspace => (self.caret.saturating_sub(1), self.caret),
                        Edit::Delete => (self.caret, (self.caret + 1).min(total)),
                        Edit::DeleteSelection => (self.caret, self.caret),
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
            Edit::Undo | Edit::Redo => unreachable!("handled before native edits"),
        }
        let changed = next != value;
        if changed {
            Self::push_snapshot(&mut self.undo, previous);
            self.redo.clear();
            self.observed_value.clone_from(&next);
        }
        (next, changed)
    }
}

/// Keep a single-line Unicode caret visible in its sized box. A whole-scalar
/// scroll origin avoids splitting UTF-8 and keeps first-party hit geometry and
/// painted glyphs on the same visible run.
pub(super) fn reveal_input_caret(
    value: &str,
    caret: usize,
    previous_start: usize,
    available: i32,
    masked: bool,
    mut measure: impl FnMut(&str) -> i32,
) -> usize {
    let chars: Vec<char> = if masked {
        vec!['•'; value.chars().count().min(MAX_TEXT_SCALARS)]
    } else {
        value.chars().take(MAX_TEXT_SCALARS).collect()
    };
    let caret = caret.min(chars.len());
    let start = previous_start.min(caret);
    let measure_range = |start: usize, end: usize, measure: &mut dyn FnMut(&str) -> i32| {
        measure(&chars[start..end].iter().collect::<String>()).max(0)
    };
    if measure_range(0, chars.len(), &mut measure) <= available.max(0) {
        return 0;
    }
    if measure_range(start, caret, &mut measure) <= available.max(0) {
        return start;
    }
    let (mut low, mut high) = (start, caret);
    while low < high {
        let mid = low + (high - low) / 2;
        if measure_range(mid, caret, &mut measure) <= available.max(0) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

/// Resolve scalar caret positions against the same soft/hard visual
/// rows used by the first-party layout compositor.
pub(super) fn textarea_caret_row(lines: &[op_layout::TextareaVisualLine], caret: usize) -> usize {
    lines
        .iter()
        .rposition(|line| line.start <= caret)
        .unwrap_or(0)
}

/// Reveal the visual row under the caret within a fixed-height textarea.
pub(super) fn reveal_textarea_row(
    lines: &[op_layout::TextareaVisualLine],
    caret: usize,
    previous_row: usize,
    height: i32,
    font_size: i32,
) -> usize {
    let visible = (height.max(1) / font_size.saturating_add(4).max(1)).max(1) as usize;
    let total = lines.len().max(1);
    if total <= visible {
        return 0;
    }
    let row = textarea_caret_row(lines, caret);
    let previous_row = previous_row.min(total.saturating_sub(visible));
    if row < previous_row {
        row
    } else if row >= previous_row.saturating_add(visible) {
        row.saturating_add(1).saturating_sub(visible)
    } else {
        previous_row
    }
}

/// Translate a clicked visible visual row back to a Unicode scalar index.
pub(super) fn wrapped_textarea_caret_at_point(
    value: &str,
    lines: &[op_layout::TextareaVisualLine],
    x: i32,
    y: i32,
    font_size: i32,
    measure: impl FnMut(&str) -> i32,
) -> usize {
    if lines.is_empty() {
        return 0;
    }
    let wanted = y.max(0) as usize / font_size.saturating_add(4).max(1) as usize;
    let line = lines[wanted.min(lines.len() - 1)];
    let text: String = value
        .chars()
        .skip(line.start)
        .take(line.end - line.start)
        .collect();
    line.start + caret_at_point(&text, x, 0, font_size, false, measure)
}

/// Selected glyph spans use visual wrapped rows, not explicit newline rows.
pub(super) fn wrapped_textarea_selection_fragments(
    value: &str,
    lines: &[op_layout::TextareaVisualLine],
    start: usize,
    end: usize,
    font_size: i32,
    mut measure: impl FnMut(&str) -> i32,
) -> Vec<SelectionFragment> {
    let (start, end) = (start.min(end), start.max(end));
    let mut fragments = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let from = start.max(line.start).min(line.end);
        let to = end.max(line.start).min(line.end);
        if from >= to {
            continue;
        }
        let prefix: String = value
            .chars()
            .skip(line.start)
            .take(from - line.start)
            .collect();
        let selected: String = value.chars().skip(from).take(to - from).collect();
        let through: String = value
            .chars()
            .skip(line.start)
            .take(to - line.start)
            .collect();
        let x = measure(&prefix).max(0);
        fragments.push(SelectionFragment {
            x,
            y: (row as i32).saturating_mul(font_size.saturating_add(4).max(1)),
            width: measure(&through).saturating_sub(x).max(1),
            height: font_size.max(1),
            text: selected,
        });
    }
    fragments
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

/// One visible selection run. Coordinates are relative to the provisional
/// first-party form control text origin, not document or screen coordinates.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SelectionFragment {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
}

/// Convert the Unicode-scalar selection to measured highlight runs, preserving
/// explicit textarea newlines. Empty ranges emit no highlight. This deliberately
/// does not pretend to implement bidi shaping or soft-wrapped textarea lines.
pub(super) fn selection_fragments(
    value: &str,
    start: usize,
    end: usize,
    font_size: i32,
    masked: bool,
    mut measure: impl FnMut(&str) -> i32,
) -> Vec<SelectionFragment> {
    let (start, end) = (start.min(end), start.max(end));
    if start == end {
        return Vec::new();
    }
    let mut fragments = Vec::new();
    let mut offset = 0usize;
    let line_height = font_size.saturating_add(4).max(1);
    for (row, line) in value.split('\n').enumerate() {
        let length = line.chars().count();
        let from = start.saturating_sub(offset).min(length);
        let to = end.saturating_sub(offset).min(length);
        if from < to {
            let shown = if masked {
                "•".repeat(length)
            } else {
                line.to_owned()
            };
            let prefix = shown.chars().take(from).collect::<String>();
            let selected = shown.chars().skip(from).take(to - from).collect::<String>();
            let selected_end = shown.chars().take(to).collect::<String>();
            let x = measure(&prefix).max(0);
            let width = measure(&selected_end).saturating_sub(x).max(1);
            fragments.push(SelectionFragment {
                x,
                y: (row as i32).saturating_mul(line_height),
                width,
                height: font_size.max(1),
                text: selected,
            });
        }
        offset = offset.saturating_add(length + 1);
        if offset > end {
            break;
        }
    }
    fragments
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
    fn textarea_row_reveal_tracks_explicit_newline_caret_without_mutating_text() {
        let value = "one\ntwo\n猫😊\nfour";
        let rows = op_layout::textarea_visual_lines(
            value,
            1000,
            16,
            &mut op_layout::ApproximateTextMeasurer,
        );
        assert_eq!(
            reveal_textarea_row(&rows, value.chars().count(), 0, 40, 16),
            2
        );
        assert_eq!(reveal_textarea_row(&rows, 7, 2, 40, 16), 1);
        assert_eq!(reveal_textarea_row(&rows, 0, 2, 40, 16), 0);
        let short = op_layout::textarea_visual_lines(
            "one\ntwo",
            1000,
            16,
            &mut op_layout::ApproximateTextMeasurer,
        );
        assert_eq!(reveal_textarea_row(&short, 7, 12, 40, 16), 0);
        assert_eq!(
            reveal_textarea_row(&rows, value.chars().count(), 0, 10, 16),
            3
        );
    }

    #[test]
    fn wrapping_visual_lines_keep_unicode_caret_click_and_selection_aligned() {
        let value = "AB猫😊EF\nxy";
        let rows = op_layout::textarea_visual_lines(
            value,
            21,
            16,
            &mut op_layout::ApproximateTextMeasurer,
        );
        assert_eq!(rows[0], op_layout::TextareaVisualLine { start: 0, end: 2 });
        assert_eq!(rows[1], op_layout::TextareaVisualLine { start: 2, end: 4 });
        assert_eq!(rows[2], op_layout::TextareaVisualLine { start: 4, end: 6 });
        assert_eq!(rows[3], op_layout::TextareaVisualLine { start: 7, end: 9 });
        assert_eq!(textarea_caret_row(&rows, 2), 1);
        assert_eq!(textarea_caret_row(&rows, 6), 2);
        assert_eq!(textarea_caret_row(&rows, 7), 3);
        let width = |s: &str| s.chars().count() as i32 * 9;
        assert_eq!(
            wrapped_textarea_caret_at_point(value, &rows, 0, 25, 16, width),
            2
        );
        assert_eq!(
            wrapped_textarea_caret_at_point(value, &rows, 100, 25, 16, width),
            4
        );
        assert_eq!(reveal_textarea_row(&rows, 7, 0, 40, 16), 2);
        let spans = wrapped_textarea_selection_fragments(value, &rows, 1, 5, 16, width);
        assert_eq!(
            spans.iter().map(|f| f.text.as_str()).collect::<Vec<_>>(),
            vec!["B", "猫😊", "E"]
        );
        assert_eq!(
            spans.iter().map(|f| f.y).collect::<Vec<_>>(),
            vec![0, 20, 40]
        );
    }

    #[test]
    fn horizontal_input_scroll_keeps_unicode_caret_visible_and_resets_at_start() {
        let width = |s: &str| (s.chars().count() as i32) * 10;
        let value = "A😊猫BCDEFG";
        let right = reveal_input_caret(value, 9, 0, 35, false, width);
        assert_eq!(right, 6);
        assert_eq!(reveal_input_caret(value, 8, right, 35, false, width), right);
        assert_eq!(reveal_input_caret(value, 0, right, 35, false, width), 0);
        assert_eq!(reveal_input_caret("ab", 2, right, 35, false, width), 0);
        assert_eq!(reveal_input_caret("abcdef", 6, 0, 25, true, width), 4);
        assert_eq!(reveal_input_caret("abcdef", 1, 4, 25, true, width), 1);
    }

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
    fn selection_highlights_visible_unicode_ranges_and_multiline_rows() {
        let width = |text: &str| text.chars().count() as i32 * 10;
        assert!(selection_fragments("A😊B", 1, 1, 16, false, width).is_empty());
        assert_eq!(
            selection_fragments("A😊B", 1, 2, 16, false, width),
            vec![SelectionFragment {
                x: 10,
                y: 0,
                width: 10,
                height: 16,
                text: "😊".into(),
            }]
        );
        assert_eq!(
            selection_fragments("ab\n猫😊Z", 1, 5, 16, false, width),
            vec![
                SelectionFragment {
                    x: 10,
                    y: 0,
                    width: 10,
                    height: 16,
                    text: "b".into(),
                },
                SelectionFragment {
                    x: 0,
                    y: 20,
                    width: 20,
                    height: 16,
                    text: "猫😊".into(),
                },
            ]
        );
        assert_eq!(
            selection_fragments("secret", 1, 3, 16, true, width),
            vec![SelectionFragment {
                x: 10,
                y: 0,
                width: 20,
                height: 16,
                text: "••".into(),
            }]
        );
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

    #[test]
    fn bounded_undo_redo_restores_unicode_selection_and_invalidates_redo() {
        let mut editor = TextEditor::new(op_dom::Document::new().root(), "A😊B");
        editor.set_utf16_selection("A😊B", 1, 3, true);
        let (value, changed) = editor.edit("A😊B", Edit::Insert("猫".into()));
        assert!(changed);
        assert_eq!(value, "A猫B");
        let (value, changed) = editor.edit(&value, Edit::Undo);
        assert!(changed);
        assert_eq!(value, "A😊B");
        assert_eq!(editor.selection_utf16(&value), (1, 3, true));
        let (value, _) = editor.edit(&value, Edit::Redo);
        assert_eq!(value, "A猫B");
        assert_eq!(editor.selection_utf16(&value), (2, 2, false));
        let (value, _) = editor.edit(&value, Edit::Undo);
        let (value, _) = editor.edit(&value, Edit::Insert("X".into()));
        assert_eq!(value, "AXB");
        assert_eq!(editor.edit(&value, Edit::Redo), (value, false));
    }

    #[test]
    fn undo_history_is_bounded_and_script_value_changes_invalidate_it() {
        let mut editor = TextEditor::new(op_dom::Document::new().root(), "");
        let mut value = String::new();
        for _ in 0..(MAX_UNDO_STEPS + 5) {
            value = editor.edit(&value, Edit::Insert("x".into())).0;
        }
        for _ in 0..MAX_UNDO_STEPS {
            value = editor.edit(&value, Edit::Undo).0;
        }
        assert_eq!(value, "xxxxx");
        assert_eq!(editor.edit(&value, Edit::Undo), (value.clone(), false));
        editor.observe_value("script");
        assert_eq!(editor.edit("script", Edit::Redo), ("script".into(), false));
    }
}

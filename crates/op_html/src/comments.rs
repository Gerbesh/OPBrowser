use crate::Tokenizer;

#[derive(Clone, Copy)]
enum CommentState {
    Start,
    StartDash,
    Body,
    Less,
    LessBang,
    LessBangDash,
    EndDash,
    End,
    EndBang,
}

impl Tokenizer {
    // WHATWG comment states; called immediately after the opening <!--.
    // Pending closing punctuation is omitted at EOF, as in the tokenization rules.
    pub(super) fn consume_comment(&mut self) -> String {
        use CommentState::*;
        let mut state = Start;
        let mut data = String::new();
        while let Some(character) = self.next_char() {
            match (state, character) {
                (Start, '-') => state = StartDash,
                (Start | StartDash | End | EndBang, '>') => break,
                (Start, _) => {
                    self.reconsume();
                    state = Body;
                }
                (StartDash | EndDash, '-') => state = End,
                (StartDash | EndDash, _) => {
                    data.push('-');
                    self.reconsume();
                    state = Body;
                }
                (Body, '<') => {
                    data.push('<');
                    state = Less;
                }
                (Body, '-') => state = EndDash,
                (Body, '\0') => data.push('\u{fffd}'),
                (Body, _) => data.push(character),
                (Less, '!') => {
                    data.push('!');
                    state = LessBang;
                }
                (Less, '<') => data.push('<'),
                (LessBang, '-') => state = LessBangDash,
                (Less | LessBang, _) => {
                    self.reconsume();
                    state = Body;
                }
                (LessBangDash, '-') => state = End,
                (LessBangDash, _) => {
                    self.reconsume();
                    state = EndDash;
                }
                (End, '!') => state = EndBang,
                (End, '-') => data.push('-'),
                (End, _) => {
                    data.push_str("--");
                    self.reconsume();
                    state = Body;
                }
                (EndBang, '-') => {
                    data.push_str("--!");
                    state = EndDash;
                }
                (EndBang, _) => {
                    data.push_str("--!");
                    self.reconsume();
                    state = Body;
                }
            }
        }
        data
    }
}

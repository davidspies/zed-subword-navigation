use zed_extension_api::{
    self as zed, EditorCommandContext, EditorCommandResult, EditorEdit, EditorSelection, Range,
};

struct SubwordNavigation;

impl zed::Extension for SubwordNavigation {
    fn new() -> Self {
        Self
    }

    fn run_editor_command(
        &mut self,
        command_id: String,
        context: EditorCommandContext,
    ) -> zed::Result<Option<EditorCommandResult>> {
        let result = match command_id.as_str() {
            "subwordNavigation.cursorSubwordLeft" => {
                move_subword(&context, Direction::Left, SelectionMode::Move)?
            }
            "subwordNavigation.cursorSubwordRight" => {
                move_subword(&context, Direction::Right, SelectionMode::Move)?
            }
            "subwordNavigation.cursorSubwordLeftSelect" => {
                move_subword(&context, Direction::Left, SelectionMode::Select)?
            }
            "subwordNavigation.cursorSubwordRightSelect" => {
                move_subword(&context, Direction::Right, SelectionMode::Select)?
            }
            "subwordNavigation.deleteSubwordLeft" => delete_subword(&context, Direction::Left)?,
            "subwordNavigation.deleteSubwordRight" => delete_subword(&context, Direction::Right)?,
            _ => return Ok(None),
        };

        Ok(Some(result))
    }
}

zed::register_extension!(SubwordNavigation);

#[derive(Clone, Copy)]
enum Direction {
    Left,
    Right,
}

#[derive(Clone, Copy)]
enum SelectionMode {
    Move,
    Select,
}

fn move_subword(
    context: &EditorCommandContext,
    direction: Direction,
    mode: SelectionMode,
) -> zed::Result<EditorCommandResult> {
    let selections = context
        .selections
        .iter()
        .map(|selection| {
            let head = selection_head(*selection);
            let boundary = next_boundary(&context.text, head, direction)?;
            match mode {
                SelectionMode::Move => Ok(EditorSelection {
                    start: boundary as u64,
                    end: boundary as u64,
                    reversed: false,
                }),
                SelectionMode::Select => Ok(selection_from_tail_and_head(
                    selection_tail(*selection),
                    boundary,
                )),
            }
        })
        .collect::<zed::Result<Vec<_>>>()?;

    Ok(EditorCommandResult {
        edits: Vec::new(),
        selections: Some(selections),
    })
}

fn delete_subword(
    context: &EditorCommandContext,
    direction: Direction,
) -> zed::Result<EditorCommandResult> {
    let mut edits = Vec::with_capacity(context.selections.len());
    let mut cursor_offsets = Vec::with_capacity(context.selections.len());

    for selection in &context.selections {
        let start = usize::try_from(selection.start)
            .map_err(|_| "Selection start exceeded usize".to_string())?;
        let end = usize::try_from(selection.end)
            .map_err(|_| "Selection end exceeded usize".to_string())?;
        validate_offset(&context.text, start)?;
        validate_offset(&context.text, end)?;

        let range = if start == end {
            let boundary = next_boundary(&context.text, selection_head(*selection), direction)?;
            cmp_range(selection_head(*selection), boundary)
        } else {
            start..end
        };

        let range_start = range.start;
        edits.push(EditorEdit {
            range: Range {
                start: u32::try_from(range.start)
                    .map_err(|_| "Edit range start exceeded u32".to_string())?,
                end: u32::try_from(range.end)
                    .map_err(|_| "Edit range end exceeded u32".to_string())?,
            },
            new_text: String::new(),
        });
        cursor_offsets.push(range_start);
    }

    let selections = cursor_offsets
        .into_iter()
        .map(|offset| {
            let offset = remap_offset(offset as u64, &edits)?;
            Ok(EditorSelection {
                start: offset,
                end: offset,
                reversed: false,
            })
        })
        .collect::<zed::Result<Vec<_>>>()?;

    Ok(EditorCommandResult {
        edits,
        selections: Some(selections),
    })
}

fn selection_head(selection: EditorSelection) -> usize {
    if selection.reversed {
        selection.start as usize
    } else {
        selection.end as usize
    }
}

fn selection_tail(selection: EditorSelection) -> usize {
    if selection.reversed {
        selection.end as usize
    } else {
        selection.start as usize
    }
}

fn selection_from_tail_and_head(tail: usize, head: usize) -> EditorSelection {
    if head < tail {
        EditorSelection {
            start: head as u64,
            end: tail as u64,
            reversed: true,
        }
    } else {
        EditorSelection {
            start: tail as u64,
            end: head as u64,
            reversed: false,
        }
    }
}

fn cmp_range(start: usize, end: usize) -> std::ops::Range<usize> {
    if start <= end {
        start..end
    } else {
        end..start
    }
}

fn next_boundary(text: &str, offset: usize, direction: Direction) -> zed::Result<usize> {
    validate_offset(text, offset)?;
    match direction {
        Direction::Left => Ok(next_boundary_left(text, offset)),
        Direction::Right => Ok(next_boundary_right(text, offset)),
    }
}

fn validate_offset(text: &str, offset: usize) -> zed::Result<()> {
    if offset <= text.len() && text.is_char_boundary(offset) {
        Ok(())
    } else {
        Err("Selection offset was not on a valid text boundary".to_string())
    }
}

fn next_boundary_left(text: &str, offset: usize) -> usize {
    let current_line_start = line_start(text, offset);

    if offset > current_line_start {
        for boundary in previous_char_boundaries(text, current_line_start, offset) {
            if boundary == current_line_start || is_boundary(text, boundary) {
                return boundary;
            }
        }
    }

    if current_line_start == 0 {
        return offset;
    }

    let previous_line_end = current_line_start - '\n'.len_utf8();
    let previous_line_start = line_start(text, previous_line_end);
    for (index, character) in text[previous_line_start..previous_line_end]
        .char_indices()
        .rev()
    {
        if !character.is_whitespace() {
            return previous_line_start + index + character.len_utf8();
        }
    }

    previous_line_start
}

fn next_boundary_right(text: &str, offset: usize) -> usize {
    let current_line_end = line_end(text, offset);

    if offset < current_line_end {
        let mut boundary = next_char_boundary(text, offset);
        while boundary <= current_line_end {
            if boundary == current_line_end || is_boundary(text, boundary) {
                return boundary;
            }
            boundary = next_char_boundary(text, boundary);
        }
    }

    if current_line_end >= text.len() {
        return current_line_end;
    }

    let next_line_start = current_line_end + '\n'.len_utf8();
    next_line_start + first_non_whitespace_offset(&text[next_line_start..])
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index)
}

fn first_non_whitespace_offset(text: &str) -> usize {
    text.char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(text.len())
}

fn previous_char_boundaries(text: &str, start: usize, end: usize) -> Vec<usize> {
    let mut boundaries = text[start..end]
        .char_indices()
        .map(|(index, _)| start + index)
        .collect::<Vec<_>>();
    boundaries.reverse();
    boundaries
}

fn next_char_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |character| offset + character.len_utf8())
}

fn is_boundary(text: &str, offset: usize) -> bool {
    let previous = character_class(previous_character(text, offset));
    let current = character_class(current_character(text, offset));
    let next = character_class(next_character(text, offset));

    if previous.separator != current.separator {
        return true;
    }
    if current.underscore && !previous.underscore {
        return true;
    }
    if previous.underscore && !current.underscore {
        return true;
    }
    if current.numeric && !previous.numeric {
        return true;
    }
    if previous.numeric && !current.numeric {
        return true;
    }
    if current.upper && previous.lower {
        return true;
    }
    if current.upper && next.lower {
        return true;
    }

    false
}

#[derive(Default)]
struct CharacterClass {
    upper: bool,
    lower: bool,
    numeric: bool,
    underscore: bool,
    separator: Option<char>,
}

fn character_class(character: Option<char>) -> CharacterClass {
    let Some(character) = character else {
        return CharacterClass::default();
    };

    if character == '_' {
        CharacterClass {
            underscore: true,
            ..CharacterClass::default()
        }
    } else if is_separator(character) {
        CharacterClass {
            separator: Some(character),
            ..CharacterClass::default()
        }
    } else if character.is_ascii_digit() {
        CharacterClass {
            numeric: true,
            ..CharacterClass::default()
        }
    } else if character.is_uppercase() && !character.is_lowercase() {
        CharacterClass {
            upper: true,
            ..CharacterClass::default()
        }
    } else if character.is_lowercase() && !character.is_uppercase() {
        CharacterClass {
            lower: true,
            ..CharacterClass::default()
        }
    } else {
        CharacterClass::default()
    }
}

fn previous_character(text: &str, offset: usize) -> Option<char> {
    text[..offset].chars().next_back()
}

fn current_character(text: &str, offset: usize) -> Option<char> {
    text[offset..].chars().next()
}

fn next_character(text: &str, offset: usize) -> Option<char> {
    current_character(text, offset).and_then(|character| {
        let next_offset = offset + character.len_utf8();
        text[next_offset..].chars().next()
    })
}

fn is_separator(character: char) -> bool {
    matches!(
        character,
        '`' | '~'
            | '!'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '('
            | ')'
            | '-'
            | '='
            | '+'
            | '['
            | '{'
            | ']'
            | '}'
            | '\\'
            | '|'
            | ';'
            | ':'
            | '\''
            | '"'
            | ','
            | '.'
            | '<'
            | '>'
            | '/'
            | '?'
    ) || character.is_whitespace()
}

fn remap_offset(offset: u64, edits: &[EditorEdit]) -> zed::Result<u64> {
    let mut edits = edits.iter().collect::<Vec<_>>();
    edits.sort_by_key(|edit| edit.range.start);

    let mut delta = 0i64;
    for edit in edits {
        let edit_start = u64::from(edit.range.start);
        let edit_end = u64::from(edit.range.end);
        if offset < edit_start {
            break;
        }

        let old_len = edit_end
            .checked_sub(edit_start)
            .ok_or_else(|| "Edit range end preceded start".to_string())?;
        let new_len = u64::try_from(edit.new_text.len())
            .map_err(|_| "Edit replacement length exceeded u64".to_string())?;

        if offset == edit_start {
            return apply_delta(edit_start, delta);
        }

        if offset < edit_end {
            let mapped = edit_start
                .checked_add(new_len)
                .ok_or_else(|| "Mapped selection offset overflowed".to_string())?;
            return apply_delta(mapped, delta);
        }

        delta = delta
            .checked_add(
                i64::try_from(new_len)
                    .map_err(|_| "Edit replacement length exceeded i64".to_string())?
                    - i64::try_from(old_len)
                        .map_err(|_| "Edit range length exceeded i64".to_string())?,
            )
            .ok_or_else(|| "Selection offset delta overflowed".to_string())?;
    }

    apply_delta(offset, delta)
}

fn apply_delta(offset: u64, delta: i64) -> zed::Result<u64> {
    if delta >= 0 {
        offset
            .checked_add(delta as u64)
            .ok_or_else(|| "Mapped selection offset overflowed".to_string())
    } else {
        offset
            .checked_sub(delta.unsigned_abs())
            .ok_or_else(|| "Mapped selection offset underflowed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(text: &str, selections: Vec<EditorSelection>) -> EditorCommandContext {
        EditorCommandContext {
            text: text.to_string(),
            selections,
            language: Some("Rust".to_string()),
            path: None,
        }
    }

    fn cursor(offset: usize) -> EditorSelection {
        EditorSelection {
            start: offset as u64,
            end: offset as u64,
            reversed: false,
        }
    }

    fn selection(start: usize, end: usize, reversed: bool) -> EditorSelection {
        EditorSelection {
            start: start as u64,
            end: end as u64,
            reversed,
        }
    }

    fn assert_selection(actual: EditorSelection, expected: EditorSelection) {
        assert_eq!(actual.start, expected.start);
        assert_eq!(actual.end, expected.end);
        assert_eq!(actual.reversed, expected.reversed);
    }

    fn apply(text: &str, result: &EditorCommandResult) -> String {
        let mut output = text.to_string();
        let mut edits = result.edits.clone();
        edits.sort_by_key(|edit| edit.range.start);
        for edit in edits.into_iter().rev() {
            output.replace_range(
                edit.range.start as usize..edit.range.end as usize,
                &edit.new_text,
            );
        }
        output
    }

    #[test]
    fn moves_through_camel_case() {
        let ctx = context("someHTTPServer42", vec![cursor(0)]);

        let result = move_subword(&ctx, Direction::Right, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(4));

        let ctx = context("someHTTPServer42", vec![cursor(8)]);
        let result = move_subword(&ctx, Direction::Right, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(14));

        let ctx = context("someHTTPServer42", vec![cursor(16)]);
        let result = move_subword(&ctx, Direction::Left, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(14));
    }

    #[test]
    fn moves_through_snake_case() {
        let ctx = context("some_http_server", vec![cursor(0)]);
        let result = move_subword(&ctx, Direction::Right, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(4));

        let ctx = context("some_http_server", vec![cursor(5)]);
        let result = move_subword(&ctx, Direction::Left, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(4));
    }

    #[test]
    fn extends_selection_from_anchor() {
        let ctx = context("someValue", vec![cursor(0)]);
        let result = move_subword(&ctx, Direction::Right, SelectionMode::Select).unwrap();
        assert_selection(result.selections.unwrap()[0], selection(0, 4, false));

        let ctx = context("someValue", vec![selection(0, 4, false)]);
        let result = move_subword(&ctx, Direction::Left, SelectionMode::Select).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(0));
    }

    #[test]
    fn deletes_empty_selection_to_boundary() {
        let ctx = context("someValue", vec![cursor(4)]);
        let result = delete_subword(&ctx, Direction::Left).unwrap();

        assert_eq!(apply(&ctx.text, &result), "Value");
        assert_selection(result.selections.unwrap()[0], cursor(0));
    }

    #[test]
    fn deletes_non_empty_selection() {
        let ctx = context("someValue", vec![selection(0, 4, false)]);
        let result = delete_subword(&ctx, Direction::Right).unwrap();

        assert_eq!(apply(&ctx.text, &result), "Value");
        assert_selection(result.selections.unwrap()[0], cursor(0));
    }

    #[test]
    fn jumps_between_lines() {
        let ctx = context("first\n    second", vec![cursor(5)]);
        let result = move_subword(&ctx, Direction::Right, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(10));

        let ctx = context("first\n    second", vec![cursor(10)]);
        let result = move_subword(&ctx, Direction::Left, SelectionMode::Move).unwrap();
        assert_selection(result.selections.unwrap()[0], cursor(6));
    }
}

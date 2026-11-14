/// Match literal text with Unicode letter, digit and underscore token boundaries.
pub fn contains_whole_token(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let is_token_character = |character: char| character.is_alphanumeric() || character == '_';
    let mut search_start = 0;
    while let Some(relative_start) = text[search_start..].find(needle) {
        let start = search_start + relative_start;
        let end = start + needle.len();
        if !text[..start]
            .chars()
            .next_back()
            .is_some_and(is_token_character)
            && !text[end..].chars().next().is_some_and(is_token_character)
        {
            return true;
        }
        let Some(next_character) = text[start..].chars().next() else {
            break;
        };
        search_start = start + next_character.len_utf8();
    }
    false
}

#[cfg(test)]
#[path = "text_token_matching_test.rs"]
mod tests;

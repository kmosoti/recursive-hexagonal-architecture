/// GitHub-style slug, as the registered markdown contract states it: Unicode
/// lowercase, keep letters, digits, spaces, `-` and `_`, then each space
/// becomes `-`, with no collapsing. **No NFC:** a combining mark is neither a
/// letter nor a digit, so `Cafe\u{301}` becomes `cafe` (corpus site MD051;
/// CHG-005 decision `slug-without-nfc`). Page ids are NFC; slugs are not.
#[must_use]
pub fn slugify(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn the_contract_examples_hold() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("a  b"), "a--b");
        assert_eq!(slugify("C++ and `code`"), "c-and-code");
        // A combining acute is dropped; a precomposed é is a letter and kept.
        assert_eq!(slugify("Caf\u{0065}\u{0301} snake_case"), "cafe-snake_case");
        assert_eq!(slugify("Caf\u{e9}"), "caf\u{e9}");
        assert_eq!(
            slugify("\u{65e5}\u{672c}\u{8a9e}"),
            "\u{65e5}\u{672c}\u{8a9e}"
        );
    }
}

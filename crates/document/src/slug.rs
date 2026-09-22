use unicode_normalization::UnicodeNormalization as _;

/// GitHub-style slug (the registered contract): NFC, Unicode lowercase, keep
/// letters, digits, spaces, `-` and `_`, then each space becomes `-`, with no
/// collapsing.
#[must_use]
pub fn slugify(text: &str) -> String {
    text.nfc()
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
        assert_eq!(
            slugify("Caf\u{0065}\u{0301} snake_case"),
            "caf\u{e9}-snake_case"
        );
        assert_eq!(
            slugify("\u{65e5}\u{672c}\u{8a9e}"),
            "\u{65e5}\u{672c}\u{8a9e}"
        );
    }
}

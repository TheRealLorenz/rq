pub fn unquote(input: &str) -> &str {
    for c in ['\'', '"'] {
        if input.starts_with(c) && input.ends_with(c) {
            return input.trim_matches(c);
        }
    }

    input
}

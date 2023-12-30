pub fn unquote(input: String) -> String {
    for c in ['\'', '"'] {
        if input.starts_with(c) && input.ends_with(c) {
            return input.trim_matches(c).to_string();
        }
    }

    input
}

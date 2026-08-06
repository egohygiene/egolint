#![forbid(unsafe_code)]

/// Formats a greeting for the supplied name.
#[must_use]
pub fn format_greeting(name: &str) -> String {
    let normalized_name = name.trim();

    if normalized_name.is_empty() {
        return String::from("Hello.");
    }

    format!("Hello, {normalized_name}.")
}

#[cfg(test)]
mod tests {
    use super::format_greeting;

    #[test]
    fn formats_a_named_greeting() {
        assert_eq!(
            format_greeting("Ego Hygiene"),
            "Hello, Ego Hygiene.",
        );
    }
}


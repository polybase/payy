pub(crate) fn to_pascal_case(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in input.chars() {
        if ch == '_' || ch == '-' {
            capitalize = true;
            continue;
        }
        if capitalize {
            out.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("foo-bar"), "FooBar");
        assert_eq!(to_pascal_case("simple"), "Simple");
        assert_eq!(to_pascal_case(""), "");
        assert_eq!(to_pascal_case("a_b_c"), "ABC");
        assert_eq!(to_pascal_case("snake_case_string"), "SnakeCaseString");
        assert_eq!(to_pascal_case("kebab-case-string"), "KebabCaseString");
        assert_eq!(to_pascal_case("mixed_case-string"), "MixedCaseString");
        assert_eq!(to_pascal_case("already_Pascal"), "AlreadyPascal");
        assert_eq!(
            to_pascal_case("___multiple___underscores___"),
            "MultipleUnderscores"
        );
    }
}

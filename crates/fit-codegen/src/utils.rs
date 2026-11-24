use std::{
    io::Write,
    process::{Command, Stdio},
};

pub fn snake_to_camel_case(input: &str) -> String {
    let trimmed = input.trim_start_matches(char::is_numeric);
    trimmed
        .split('_')
        .map(|w| {
            let mut new = w.to_string();
            if new.is_empty() {
                return new;
            }
            if let Some((idx, c)) = new.char_indices().next() {
                new.replace_range(idx..idx + 1, &c.to_uppercase().to_string());
            }

            new
        })
        .collect()
}

pub fn format_code(code: &str) -> String {
    let mut child = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn rustfmt");

    // Write code to rustfmt's stdin
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(code.as_bytes())
            .expect("Failed to write to rustfmt");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to read rustfmt output");

    if output.status.success() {
        String::from_utf8(output.stdout).unwrap_or_else(|_| code.to_string())
    } else {
        println!("cargo:warning=rustfmt failed, using unformatted code");
        code.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_snake_case() {
        assert_eq!(snake_to_camel_case("hello_world"), "HelloWorld");
        assert_eq!(snake_to_camel_case("foo_bar_baz"), "FooBarBaz");
        assert_eq!(snake_to_camel_case("snake_case_string"), "SnakeCaseString");
    }

    #[test]
    fn test_single_word() {
        assert_eq!(snake_to_camel_case("hello"), "Hello");
        assert_eq!(snake_to_camel_case("world"), "World");
        assert_eq!(snake_to_camel_case("a"), "A");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(snake_to_camel_case(""), "");
    }

    #[test]
    fn test_leading_numbers() {
        assert_eq!(snake_to_camel_case("123hello_world"), "HelloWorld");
        assert_eq!(snake_to_camel_case("456foo_bar"), "FooBar");
        assert_eq!(snake_to_camel_case("0test_case"), "TestCase");
        assert_eq!(snake_to_camel_case("99_hello_world"), "HelloWorld");
    }

    #[test]
    fn test_only_numbers() {
        assert_eq!(snake_to_camel_case("123456"), "");
        assert_eq!(snake_to_camel_case("0"), "");
    }

    #[test]
    fn test_consecutive_underscores() {
        assert_eq!(snake_to_camel_case("hello__world"), "HelloWorld");
        assert_eq!(snake_to_camel_case("foo___bar"), "FooBar");
        assert_eq!(snake_to_camel_case("test____case"), "TestCase");
    }

    #[test]
    fn test_leading_underscore() {
        assert_eq!(snake_to_camel_case("_hello_world"), "HelloWorld");
        assert_eq!(snake_to_camel_case("__foo_bar"), "FooBar");
    }

    #[test]
    fn test_trailing_underscore() {
        assert_eq!(snake_to_camel_case("hello_world_"), "HelloWorld");
        assert_eq!(snake_to_camel_case("foo_bar__"), "FooBar");
    }

    #[test]
    fn test_only_underscores() {
        assert_eq!(snake_to_camel_case("_"), "");
        assert_eq!(snake_to_camel_case("___"), "");
    }

    #[test]
    fn test_mixed_case_input() {
        assert_eq!(snake_to_camel_case("Hello_World"), "HelloWorld");
        assert_eq!(snake_to_camel_case("FOO_bar"), "FOOBar");
        assert_eq!(snake_to_camel_case("mixED_CaSe"), "MixEDCaSe");
    }

    #[test]
    fn test_numbers_in_words() {
        assert_eq!(snake_to_camel_case("hello2_world3"), "Hello2World3");
        assert_eq!(snake_to_camel_case("test_123_case"), "Test123Case");
        assert_eq!(snake_to_camel_case("var_1_name"), "Var1Name");
    }

    #[test]
    fn test_special_characters() {
        assert_eq!(snake_to_camel_case("hello-world_test"), "Hello-worldTest");
        assert_eq!(snake_to_camel_case("test_with@symbol"), "TestWith@symbol");
        assert_eq!(snake_to_camel_case("special_#_chars"), "Special#Chars");
    }

    #[test]
    fn test_whitespace_in_words() {
        assert_eq!(snake_to_camel_case("hello world_test"), "Hello worldTest");
        assert_eq!(snake_to_camel_case("test_with space"), "TestWith space");
    }

    #[test]
    fn test_leading_numbers_with_underscores() {
        assert_eq!(snake_to_camel_case("123_hello_world"), "HelloWorld");
        assert_eq!(snake_to_camel_case("42_test_case_name"), "TestCaseName");
    }

    #[test]
    fn test_complex_cases() {
        // Mix of leading numbers, consecutive underscores, and edge cases
        assert_eq!(snake_to_camel_case("123__hello___world_"), "HelloWorld");
        assert_eq!(snake_to_camel_case("0_a__b_"), "AB");
    }
}

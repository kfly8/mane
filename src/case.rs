use anyhow::Result;
use convert_case::{Case, Casing};

/// String naming convention (case) types
#[derive(Debug)]
#[allow(dead_code)]
pub enum StringCase {
    Pascal,     // HelloWorld
    Kebab,      // hello-world
    Camel,      // helloWorld
    ScreamingSnake, // HELLO_WORLD
    Snake,      // hello_world
    Unknown,    // other
}

/// Detect the case style of a string
/// 
/// # Arguments
/// * `s` - The string to analyze
/// 
/// # Returns
/// * `StringCase` - The detected case style
#[allow(dead_code)]
pub fn detect_case(s: &str) -> StringCase {
    if s.contains('-') {
        StringCase::Kebab
    } else if s.contains('_') {
        if s.to_uppercase() == s {
            StringCase::ScreamingSnake
        } else {
            StringCase::Snake
        }
    } else if !s.chars().next().unwrap_or(' ').is_uppercase() && s.chars().any(char::is_uppercase) {
        StringCase::Camel
    } else if s.chars().next().unwrap_or(' ').is_uppercase() && s.chars().any(char::is_lowercase) {
        StringCase::Pascal
    } else {
        StringCase::Unknown
    }
}

/// Convert a string to a specified case style
/// 
/// # Arguments
/// * `s` - The string to convert
/// * `case_type` - The target case style
/// 
/// # Returns
/// * `String` - The converted string
pub fn convert_case(s: &str, case_type: &StringCase) -> String {
    match case_type {
        StringCase::Pascal => s.to_case(Case::Pascal),
        StringCase::Kebab => s.to_case(Case::Kebab),
        StringCase::Camel => s.to_case(Case::Camel),
        StringCase::ScreamingSnake => s.to_case(Case::UpperSnake),
        StringCase::Snake => s.to_case(Case::Snake),
        StringCase::Unknown => s.to_string(),
    }
}

/// Replace strings while considering multiple case variants
/// 
/// # Arguments
/// * `content` - The content to replace in
/// * `from` - The string to replace
/// * `to` - The replacement string
/// 
/// # Returns
/// * `Result<String>` - The replaced content
pub fn replace_with_case_variants(content: &str, from: &str, to: &str) -> Result<String> {
    use std::sync::atomic::Ordering;
    use crate::args::GLOBAL_CASE_ENABLED;
    
    let mut result = content.to_string();
    
    // Direct replacement (original case)
    result = result.replace(from, to);
    
    // If case transformation is enabled, handle different case variants
    if GLOBAL_CASE_ENABLED.load(Ordering::Relaxed) {
        // For each case variant, create and apply replacements, including the current case
        // This ensures we apply transformations for all cases, not just the ones different from the original
        let case_variants = [
            StringCase::Pascal,
            StringCase::Kebab,
            StringCase::Camel,
            StringCase::ScreamingSnake,
            StringCase::Snake,
        ];
        
        for case_type in &case_variants {
            // Skip if this is exactly the same as the original input string to avoid redundant replacements
            // (Not skipping based on case types, which was causing issues with mixed casing)
            let from_variant = convert_case(from, case_type);
            
            // Skip if converting to this case gives the same string as original
            // or if the variant is empty
            if from_variant == from || from_variant.is_empty() {
                continue;
            }
            
            // Make sure the from_variant actually exists in the original content
            if !content.contains(&from_variant) {
                continue;
            }
            
            // Convert the 'to' string to the same case variant
            let to_variant = convert_case(to, case_type);
            
            // Apply this case-specific replacement
            result = result.replace(&from_variant, &to_variant);
        }
    }
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_case_detection() {
        assert!(matches!(detect_case("HelloWorld"), StringCase::Pascal));
        assert!(matches!(detect_case("hello-world"), StringCase::Kebab));
        assert!(matches!(detect_case("helloWorld"), StringCase::Camel));
        assert!(matches!(detect_case("HELLO_WORLD"), StringCase::ScreamingSnake));
        assert!(matches!(detect_case("hello_world"), StringCase::Snake));
    }
    
    #[test]
    fn test_case_conversion() {
        // Pascal case conversions
        assert_eq!(convert_case("HelloWorld", &StringCase::Pascal), "HelloWorld");
        assert_eq!(convert_case("hello-world", &StringCase::Pascal), "HelloWorld");
        assert_eq!(convert_case("helloWorld", &StringCase::Pascal), "HelloWorld");
        assert_eq!(convert_case("HELLO_WORLD", &StringCase::Pascal), "HelloWorld");
        assert_eq!(convert_case("hello_world", &StringCase::Pascal), "HelloWorld");
        
        // Kebab case conversions
        assert_eq!(convert_case("HelloWorld", &StringCase::Kebab), "hello-world");
        assert_eq!(convert_case("hello-world", &StringCase::Kebab), "hello-world");
        assert_eq!(convert_case("helloWorld", &StringCase::Kebab), "hello-world");
        assert_eq!(convert_case("HELLO_WORLD", &StringCase::Kebab), "hello-world");
        assert_eq!(convert_case("hello_world", &StringCase::Kebab), "hello-world");
        
        // Camel case conversions
        assert_eq!(convert_case("HelloWorld", &StringCase::Camel), "helloWorld");
        assert_eq!(convert_case("hello-world", &StringCase::Camel), "helloWorld");
        assert_eq!(convert_case("helloWorld", &StringCase::Camel), "helloWorld");
        assert_eq!(convert_case("HELLO_WORLD", &StringCase::Camel), "helloWorld");
        assert_eq!(convert_case("hello_world", &StringCase::Camel), "helloWorld");
        
        // ScreamingSnake case conversions
        assert_eq!(convert_case("HelloWorld", &StringCase::ScreamingSnake), "HELLO_WORLD");
        assert_eq!(convert_case("hello-world", &StringCase::ScreamingSnake), "HELLO_WORLD");
        assert_eq!(convert_case("helloWorld", &StringCase::ScreamingSnake), "HELLO_WORLD");
        assert_eq!(convert_case("HELLO_WORLD", &StringCase::ScreamingSnake), "HELLO_WORLD");
        assert_eq!(convert_case("hello_world", &StringCase::ScreamingSnake), "HELLO_WORLD");
        
        // Snake case conversions
        assert_eq!(convert_case("HelloWorld", &StringCase::Snake), "hello_world");
        assert_eq!(convert_case("hello-world", &StringCase::Snake), "hello_world");
        assert_eq!(convert_case("helloWorld", &StringCase::Snake), "hello_world");
        assert_eq!(convert_case("HELLO_WORLD", &StringCase::Snake), "hello_world");
        assert_eq!(convert_case("hello_world", &StringCase::Snake), "hello_world");
    }
    
    #[test]
    fn test_replace_with_case_variants() {
        // Configure globals for testing
        use std::sync::atomic::Ordering;
        use crate::args::GLOBAL_CASE_ENABLED;
        GLOBAL_CASE_ENABLED.store(true, Ordering::Relaxed);
        
        // Test with a simple example like in the spec
        let content = "Hello, World\nhello, world";
        
        // Test replacing "Hello" with "Hi"
        let result = replace_with_case_variants(content, "Hello", "Hi").unwrap();
        assert!(result.contains("Hi, World"));
        assert!(result.contains("hi, world"));
        
        // Test replacing "hello" with "hi" - use a fresh content string to avoid
        // being affected by previous replacements
        let content2 = "Hello, World\nhello, world";
        let result2 = replace_with_case_variants(content2, "hello", "hi").unwrap();
        // Check that both forms were replaced
        assert!(result2.contains("Hi, World"));
        assert!(result2.contains("hi, world"));
        
        // Test with multiple word replacement
        let content3 = "HelloWorld helloWorld hello_world HELLO_WORLD hello-world";
        
        // Test replacing "HelloWorld" with "GoodMorning"
        let result3 = replace_with_case_variants(content3, "HelloWorld", "GoodMorning").unwrap();
        assert!(result3.contains("GoodMorning"));
        assert!(result3.contains("goodMorning"));
        assert!(result3.contains("good_morning"));
        assert!(result3.contains("GOOD_MORNING"));
        assert!(result3.contains("good-morning"));
    }
}

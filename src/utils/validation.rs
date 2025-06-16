use regex::Regex;
use html_escape::encode_text;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(
        r"^(https?://)?([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(/[a-zA-Z0-9-._~:/?#[\]@!$&'()*+,;=]*)?$"
    ).unwrap();
}

/// 验证URL是否有效
pub fn validate_url(url: &str) -> bool {
    if url.starts_with("data:image/") {
        return true;
    }
    URL_REGEX.is_match(url)
}

/// 清理输入文本，防止XSS攻击
pub fn sanitize_input(input: &str) -> String {
    encode_text(input).to_string()
}

/// 验证文件类型
pub fn validate_file_type(file_type: &str) -> bool {
    matches!(
        file_type,
        "image/jpeg" | "image/png" | "image/svg+xml" | "image/gif"
    )
}

/// 验证文件大小
pub fn validate_file_size(size: usize, max_size: usize) -> bool {
    size <= max_size
}

/// 验证标题长度
pub fn validate_title_length(title: &str, max_length: usize) -> bool {
    title.len() <= max_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com"));
        assert!(validate_url("http://example.com/path"));
        assert!(validate_url("data:image/png;base64,abc123"));
        assert!(!validate_url("invalid-url"));
        assert!(!validate_url("javascript:alert(1)"));
    }

    #[test]
    fn test_sanitize_input() {
        assert_eq!(
            sanitize_input("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(
            sanitize_input("Hello & World"),
            "Hello &amp; World"
        );
    }

    #[test]
    fn test_validate_file_type() {
        assert!(validate_file_type("image/jpeg"));
        assert!(validate_file_type("image/png"));
        assert!(validate_file_type("image/svg+xml"));
        assert!(validate_file_type("image/gif"));
        assert!(!validate_file_type("application/pdf"));
    }

    #[test]
    fn test_validate_file_size() {
        assert!(validate_file_size(1024, 2048));
        assert!(!validate_file_size(2048, 1024));
    }

    #[test]
    fn test_validate_title_length() {
        assert!(validate_title_length("Short title", 20));
        assert!(!validate_title_length("This is a very long title that exceeds the maximum length", 20));
    }
} 
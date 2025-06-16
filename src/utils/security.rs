use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use serde::{Serialize, Deserialize};

lazy_static! {
    static ref RATE_LIMITS: Mutex<HashMap<String, Vec<Instant>>> = Mutex::new(HashMap::new());
    static ref CSRF_TOKENS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_REQUESTS_PER_WINDOW: usize = 60;
const CSRF_TOKEN_EXPIRY: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub csrf_token: String,
    pub rate_limit_enabled: bool,
    pub max_requests_per_window: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            csrf_token: generate_csrf_token(),
            rate_limit_enabled: true,
            max_requests_per_window: MAX_REQUESTS_PER_WINDOW,
        }
    }
}

/// 生成CSRF令牌
pub fn generate_csrf_token() -> String {
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    
    let mut tokens = CSRF_TOKENS.lock().unwrap();
    tokens.insert(token.clone(), Instant::now());
    token
}

/// 验证CSRF令牌
pub fn validate_csrf_token(token: &str) -> bool {
    let mut tokens = CSRF_TOKENS.lock().unwrap();
    if let Some(created_at) = tokens.get(token) {
        if created_at.elapsed() < CSRF_TOKEN_EXPIRY {
            tokens.remove(token);
            return true;
        }
    }
    false
}

/// 清理过期的CSRF令牌
pub fn cleanup_expired_csrf_tokens() {
    let mut tokens = CSRF_TOKENS.lock().unwrap();
    tokens.retain(|_, created_at| created_at.elapsed() < CSRF_TOKEN_EXPIRY);
}

/// 检查请求速率限制
pub fn check_rate_limit(identifier: &str) -> Result<(), String> {
    let mut limits = RATE_LIMITS.lock().unwrap();
    let now = Instant::now();
    
    // 清理过期的请求记录
    if let Some(requests) = limits.get_mut(identifier) {
        requests.retain(|&time| now.duration_since(time) < RATE_LIMIT_WINDOW);
        
        if requests.len() >= MAX_REQUESTS_PER_WINDOW {
            return Err("Rate limit exceeded".to_string());
        }
        
        requests.push(now);
    } else {
        limits.insert(identifier.to_string(), vec![now]);
    }
    
    Ok(())
}

/// 清理过期的速率限制记录
pub fn cleanup_rate_limits() {
    let mut limits = RATE_LIMITS.lock().unwrap();
    let now = Instant::now();
    
    limits.retain(|_, requests| {
        requests.retain(|&time| now.duration_since(time) < RATE_LIMIT_WINDOW);
        !requests.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_csrf_token_generation_and_validation() {
        let token = generate_csrf_token();
        assert!(validate_csrf_token(&token));
        assert!(!validate_csrf_token(&token)); // 令牌应该只能使用一次
    }

    #[test]
    fn test_rate_limiting() {
        let identifier = "test_client";
        
        // 测试正常请求
        for _ in 0..MAX_REQUESTS_PER_WINDOW {
            assert!(check_rate_limit(identifier).is_ok());
        }
        
        // 测试超出限制
        assert!(check_rate_limit(identifier).is_err());
        
        // 等待窗口期结束
        thread::sleep(RATE_LIMIT_WINDOW);
        assert!(check_rate_limit(identifier).is_ok());
    }
} 
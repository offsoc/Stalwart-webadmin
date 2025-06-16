use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::VecDeque;

const MAX_AUDIT_LOGS: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ConfigUpdate,
    FileUpload,
    Login,
    Logout,
    ResetConfig,
    PreviewToggle,
    AutoSaveToggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub user: String,
    pub details: String,
    pub ip_address: Option<String>,
    pub success: bool,
}

lazy_static! {
    static ref AUDIT_LOGS: Mutex<VecDeque<AuditLog>> = Mutex::new(VecDeque::with_capacity(MAX_AUDIT_LOGS));
}

/// 记录审计日志
pub fn log_audit(
    action: AuditAction,
    user: &str,
    details: &str,
    ip_address: Option<String>,
    success: bool,
) {
    let log = AuditLog {
        timestamp: Utc::now(),
        action,
        user: user.to_string(),
        details: details.to_string(),
        ip_address,
        success,
    };

    let mut logs = AUDIT_LOGS.lock().unwrap();
    if logs.len() >= MAX_AUDIT_LOGS {
        logs.pop_front();
    }
    logs.push_back(log);
}

/// 获取审计日志
pub fn get_audit_logs() -> Vec<AuditLog> {
    AUDIT_LOGS.lock().unwrap().iter().cloned().collect()
}

/// 清理过期的审计日志
pub fn cleanup_audit_logs(max_age_days: i64) {
    let mut logs = AUDIT_LOGS.lock().unwrap();
    let cutoff = Utc::now() - chrono::Duration::days(max_age_days);
    logs.retain(|log| log.timestamp > cutoff);
}

/// 导出审计日志
pub fn export_audit_logs() -> String {
    let logs = get_audit_logs();
    serde_json::to_string_pretty(&logs).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logging() {
        log_audit(
            AuditAction::ConfigUpdate,
            "test_user",
            "Updated layout configuration",
            Some("127.0.0.1".to_string()),
            true,
        );

        let logs = get_audit_logs();
        assert!(!logs.is_empty());
        assert_eq!(logs[0].user, "test_user");
        assert_eq!(logs[0].action, AuditAction::ConfigUpdate);
    }

    #[test]
    fn test_audit_log_rotation() {
        // 填充超过最大限制的日志
        for i in 0..MAX_AUDIT_LOGS + 1 {
            log_audit(
                AuditAction::ConfigUpdate,
                &format!("user_{}", i),
                "Test log",
                None,
                true,
            );
        }

        let logs = get_audit_logs();
        assert_eq!(logs.len(), MAX_AUDIT_LOGS);
    }
} 
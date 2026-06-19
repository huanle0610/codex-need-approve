#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalEvent {
    pub id: String,
}

pub fn parse_approval_event(line: &str, path: &str) -> Option<ApprovalEvent> {
    let is_show_approval = line.contains("[desktop-notifications] show approval");
    let is_permission_or_question = line.contains("[desktop-notifications] show notification")
        && (line.contains("kind=permission") || line.contains("kind=question"));
    let is_approval_response = line.contains("method=item/commandExecution/requestApproval")
        && line.contains("Sending server response");

    if !is_show_approval && !is_permission_or_question && !is_approval_response {
        return None;
    }

    let raw_id = find_field(line, "notificationId=")
        .or_else(|| find_field(line, "requestId="))
        .or_else(|| find_field(line, "id="))
        .unwrap_or_else(|| fallback_id(path, line));

    Some(ApprovalEvent {
        id: normalize_id(&raw_id),
    })
}

fn find_field(line: &str, prefix: &str) -> Option<String> {
    let start = line.find(prefix)? + prefix.len();
    let value = line[start..].split_whitespace().next().unwrap_or("").trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_id(raw: &str) -> String {
    if let Some(n) = raw.strip_prefix("approval-local-") {
        return format!("approval-{n}");
    }
    if raw.chars().all(|c| c.is_ascii_digit()) {
        return format!("approval-{raw}");
    }
    raw.to_string()
}

fn fallback_id(path: &str, line: &str) -> String {
    let mut hash: u64 = 1469598103934665603;
    for b in path.bytes().chain(line.bytes()) {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("fallback-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_show_approval_and_uses_request_id() {
        let line = "2026-06-19T11:01:42.909Z info [electron-message-handler] [desktop-notifications] show approval conversationId=abc kind=commandExecution requestId=33";
        let event = parse_approval_event(line, "codex.log").expect("approval event");
        assert_eq!(event.id, "approval-33");
    }

    #[test]
    fn detects_permission_notification_and_normalizes_local_id() {
        let line = "2026-06-19T11:01:42.910Z info [desktop-notifications] show notification actionCount=3 kind=permission notificationId=approval-local-33";
        let event = parse_approval_event(line, "codex.log").expect("permission notification");
        assert_eq!(event.id, "approval-33");
    }

    #[test]
    fn detects_question_notification() {
        let line = "2026-06-19T11:01:42.910Z info [desktop-notifications] show notification actionCount=2 kind=question notificationId=question-local-99";
        let event = parse_approval_event(line, "codex.log").expect("question notification");
        assert_eq!(event.id, "question-local-99");
    }

    #[test]
    fn detects_request_approval_response() {
        let line = "2026-06-19T11:48:38.477Z info [electron-message-handler] Sending server response id=55 method=item/commandExecution/requestApproval response={\"decision\":\"accept\"}";
        let event = parse_approval_event(line, "codex.log").expect("approval response");
        assert_eq!(event.id, "approval-55");
    }

    #[test]
    fn ignores_non_approval_notifications() {
        let line = "2026-06-19T11:01:42.910Z info [desktop-notifications] show notification actionCount=0 kind=turn-complete notificationId=turn-1";
        assert_eq!(parse_approval_event(line, "codex.log"), None);
    }
}

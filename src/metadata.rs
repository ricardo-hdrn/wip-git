/// Metadata stored in the [wip-push] trailer block of the commit message
#[derive(Clone)]
pub struct WipMetadata {
    pub message: String,
    pub branch: String,
    pub task: Option<String>,
    pub files: usize,
    pub untracked: usize,
}

impl WipMetadata {
    /// Build the full commit message with trailer
    pub fn to_commit_message(&self) -> String {
        let mut msg = format!("wip: {}", self.message);
        msg.push_str("\n\n[wip-push]");
        msg.push_str(&format!("\nbranch={}", self.branch));
        if let Some(ref task) = self.task {
            msg.push_str(&format!("\ntask={task}"));
        }
        msg.push_str(&format!("\nfiles={}", self.files));
        msg.push_str(&format!("\nuntracked={}", self.untracked));
        msg
    }

    /// Parse metadata from a commit message
    pub fn from_commit_message(msg: &str) -> Self {
        let mut message = String::new();
        let mut branch = String::new();
        let mut task = None;
        let mut files = 0;
        let mut untracked = 0;

        let mut in_trailer = false;

        for line in msg.lines() {
            if line.trim() == "[wip-push]" {
                in_trailer = true;
                continue;
            }

            if in_trailer {
                if let Some(val) = line.strip_prefix("branch=") {
                    branch = val.to_string();
                } else if let Some(val) = line.strip_prefix("task=") {
                    task = Some(val.to_string());
                } else if let Some(val) = line.strip_prefix("files=") {
                    files = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("untracked=") {
                    untracked = val.parse().unwrap_or(0);
                }
            } else if let Some(m) = line.strip_prefix("wip: ") {
                message = m.to_string();
            }
        }

        WipMetadata {
            message,
            branch,
            task,
            files,
            untracked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let meta = WipMetadata {
            message: "fix auth flow".to_string(),
            branch: "main".to_string(),
            task: Some("JIRA-123".to_string()),
            files: 3,
            untracked: 1,
        };

        let msg = meta.to_commit_message();
        let parsed = WipMetadata::from_commit_message(&msg);

        assert_eq!(parsed.message, "fix auth flow");
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.task.as_deref(), Some("JIRA-123"));
        assert_eq!(parsed.files, 3);
        assert_eq!(parsed.untracked, 1);
    }

    #[test]
    fn test_parse_without_task() {
        let meta = WipMetadata {
            message: "wip".to_string(),
            branch: "feature-x".to_string(),
            task: None,
            files: 5,
            untracked: 0,
        };

        let msg = meta.to_commit_message();
        let parsed = WipMetadata::from_commit_message(&msg);

        assert_eq!(parsed.message, "wip");
        assert_eq!(parsed.branch, "feature-x");
        assert!(parsed.task.is_none());
        assert_eq!(parsed.files, 5);
        assert_eq!(parsed.untracked, 0);
    }

    #[test]
    fn test_parse_minimal() {
        let msg = "wip: stuff\n\n[wip-push]\nbranch=main\nfiles=0\nuntracked=0";
        let parsed = WipMetadata::from_commit_message(msg);

        assert_eq!(parsed.message, "stuff");
        assert_eq!(parsed.branch, "main");
        assert!(parsed.task.is_none());
        assert_eq!(parsed.files, 0);
        assert_eq!(parsed.untracked, 0);
    }
}

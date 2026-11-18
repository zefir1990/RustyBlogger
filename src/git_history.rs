use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const COMMIT_MARKER: &str = "__COMMIT__";

#[derive(Debug, Clone)]
pub struct CommitDate {
    pub timestamp: u64,
    pub date: String,
}

pub fn first_commit_dates(directory: &Path) -> Result<BTreeMap<String, CommitDate>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args([
            "log",
            "--format=__COMMIT__%ct|%cs",
            "--name-only",
            "--relative",
        ])
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        let standard_error = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git log failed with status {}: {}",
            output.status,
            standard_error.trim()
        ));
    }
    let standard_output = String::from_utf8_lossy(&output.stdout);
    let mut dates = BTreeMap::new();
    let mut current_commit_date = None;
    for line in standard_output.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(commit_text) = line.strip_prefix(COMMIT_MARKER) {
            let mut parts = commit_text.splitn(2, '|');
            let timestamp = parts.next().and_then(|value| value.parse::<u64>().ok());
            let date = parts.next().map(str::to_string);
            current_commit_date = match (timestamp, date) {
                (Some(timestamp), Some(date)) if !date.is_empty() => {
                    Some(CommitDate { timestamp, date })
                }
                _ => None,
            };
            continue;
        }
        if let Some(commit_date) = &current_commit_date {
            dates.insert(line.to_string(), commit_date.clone());
        }
    }
    Ok(dates)
}

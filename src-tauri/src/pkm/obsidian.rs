use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

pub struct TranscriptLine {
    pub speaker: String,
    /// Seconds from meeting start.
    pub at_secs: u32,
    pub text: String,
}

pub struct MeetingNote {
    pub started_at: DateTime<Local>,
    pub source: String,
    pub speakers: Vec<String>,
    pub duration_min: u32,
    pub summary: String,
    pub transcript: Vec<TranscriptLine>,
}

/// Slug from the first words of the transcript, e.g. "good-morning-everyone".
fn slug(note: &MeetingNote) -> String {
    let s: String = note
        .transcript
        .first()
        .map(|l| l.text.as_str())
        .unwrap_or("meeting")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_lowercase();
    if s.is_empty() {
        "meeting".into()
    } else {
        s
    }
}

/// Write one note per meeting: <vault>/Meetings/YYYY-MM-DD-HHmm-<slug>.md
/// following the schema in ARCHITECTURE.md.
pub fn write_meeting_note(vault: &Path, note: &MeetingNote) -> Result<PathBuf> {
    let dir = vault.join("Meetings");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!(
        "{}-{}.md",
        note.started_at.format("%Y-%m-%d-%H%M"),
        slug(note)
    ));

    let mut body = String::new();
    body.push_str("---\n");
    body.push_str(&format!("date: {}\n", note.started_at.format("%Y-%m-%dT%H:%M")));
    body.push_str(&format!("source: {}\n", note.source));
    body.push_str(&format!("speakers: [{}]\n", note.speakers.join(", ")));
    body.push_str(&format!("duration_min: {}\n", note.duration_min));
    body.push_str("tags: [meeting, ghost-capture]\n");
    body.push_str("---\n## Summary\n");
    body.push_str(note.summary.trim());
    body.push_str("\n\n## Transcript\n");
    for line in &note.transcript {
        body.push_str(&format!(
            "> **{}** ({:02}:{:02}): {}\n",
            line.speaker,
            line.at_secs / 60,
            line.at_secs % 60,
            line.text
        ));
    }

    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn writes_note_per_schema() {
        let tmp = std::env::temp_dir().join("shout-obsidian-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let note = MeetingNote {
            started_at: Local.with_ymd_and_hms(2026, 7, 2, 14, 30, 0).unwrap(),
            source: "mic".into(),
            speakers: vec!["speaker_1".into(), "speaker_2".into()],
            duration_min: 47,
            summary: "TL;DR: things happened.".into(),
            transcript: vec![TranscriptLine {
                speaker: "speaker_1".into(),
                at_secs: 2,
                text: "hello".into(),
            }],
        };
        let path = write_meeting_note(&tmp, &note).unwrap();
        assert_eq!(path.file_name().unwrap(), "2026-07-02-1430-hello.md");
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.starts_with("---\ndate: 2026-07-02T14:30\n"));
        assert!(s.contains("speakers: [speaker_1, speaker_2]"));
        assert!(s.contains("## Summary"));
        assert!(s.contains("> **speaker_1** (00:02): hello"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

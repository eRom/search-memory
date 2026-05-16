use chrono::{DateTime, Local};
use glob::glob;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

const FRAGMENT_RADIUS: usize = 120;

struct Hit {
    date: String,
    fragment: String,
    mtime: SystemTime,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: search-memories <query>");
        return ExitCode::from(2);
    }
    let query = args.join(" ");
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .collect();
    if terms.is_empty() {
        eprintln!("usage: search-memories <query>");
        return ExitCode::from(2);
    }

    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => {
            eprintln!("HOME not set");
            return ExitCode::from(1);
        }
    };
    let pattern = home.join(".claude/projects/*/memory/*.md");
    let pattern_str = pattern.to_string_lossy().into_owned();

    let mut hits: Vec<Hit> = Vec::new();
    for entry in glob(&pattern_str).expect("invalid glob pattern").flatten() {
        if let Some(hit) = scan_file(&entry, &terms) {
            hits.push(hit);
        }
    }

    if hits.is_empty() {
        println!("No matches found in memory sessions.");
        return ExitCode::SUCCESS;
    }

    hits.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    for h in hits {
        let obj = json!({
            "date": h.date,
            "fragment": h.fragment,
        });
        println!("{obj}");
    }

    ExitCode::SUCCESS
}

fn scan_file(path: &PathBuf, terms: &[String]) -> Option<Hit> {
    let content = fs::read_to_string(path).ok()?;
    let lower = content.to_lowercase();

    let mut anchor_pos = 0usize;
    let mut anchor_len = 0usize;
    for t in terms {
        match lower.find(t) {
            Some(p) => {
                if t.len() > anchor_len {
                    anchor_len = t.len();
                    anchor_pos = p;
                }
            }
            None => return None,
        }
    }

    let fragment = extract_fragment(&content, anchor_pos, terms);
    let mtime = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let date: DateTime<Local> = mtime.into();
    Some(Hit {
        date: date.format("%Y/%m/%d").to_string(),
        fragment,
        mtime,
    })
}

fn extract_fragment(content: &str, hit_byte: usize, _terms: &[String]) -> String {
    let len = content.len();
    let start = floor_char_boundary(content, hit_byte.saturating_sub(FRAGMENT_RADIUS));
    let end = ceil_char_boundary(content, (hit_byte + FRAGMENT_RADIUS).min(len));
    let slice = &content[start..end];

    let normalized: String = slice.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < len { "..." } else { "" };
    format!("{prefix}{normalized}{suffix}")
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_char_boundary(s: &str, mut i: usize) -> usize {
    let len = s.len();
    while i < len && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

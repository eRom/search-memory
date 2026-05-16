use chrono::{DateTime, Local};
use glob::glob;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

const FRAGMENT_RADIUS: usize = 140;

struct Hit {
    date: String,
    project: String,
    file: String,
    topic: String,
    fragment: String,
    mtime: SystemTime,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: search-memories <query>");
        return ExitCode::from(2);
    }
    let terms: Vec<String> = args
        .join(" ")
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

    let mut hits: Vec<Hit> = Vec::new();
    for entry in glob(&pattern.to_string_lossy()).expect("bad glob").flatten() {
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
            "project": h.project,
            "file": h.file,
            "topic": h.topic,
            "fragment": h.fragment,
        });
        println!("{obj}");
    }

    ExitCode::SUCCESS
}

fn scan_file(path: &Path, terms: &[String]) -> Option<Hit> {
    let content = fs::read_to_string(path).ok()?;
    let lower = content.to_lowercase();

    let mut term_positions: Vec<Vec<usize>> = Vec::with_capacity(terms.len());
    for t in terms {
        let positions: Vec<usize> = find_all(&lower, t);
        if positions.is_empty() {
            return None;
        }
        term_positions.push(positions);
    }

    let (win_start, win_end) = min_window(&term_positions, &terms.iter().map(|t| t.len()).collect::<Vec<_>>());
    let anchor = (win_start + win_end) / 2;
    let fragment = extract_fragment(&content, anchor);

    let mtime = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let date: DateTime<Local> = mtime.into();
    let project = extract_project(path);
    let file = path.file_name()?.to_string_lossy().into_owned();
    let topic = extract_topic(&content);

    Some(Hit {
        date: date.format("%Y/%m/%d").to_string(),
        project,
        file,
        topic,
        fragment,
        mtime,
    })
}

fn find_all(haystack: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(p) = haystack[from..].find(needle) {
        let pos = from + p;
        out.push(pos);
        from = pos + needle.len().max(1);
    }
    out
}

/// Smallest [start, end] interval containing at least one position from each term.
/// Returns the byte positions in the original string.
fn min_window(term_positions: &[Vec<usize>], term_lens: &[usize]) -> (usize, usize) {
    let mut flat: Vec<(usize, usize)> = Vec::new();
    for (idx, positions) in term_positions.iter().enumerate() {
        for &p in positions {
            flat.push((p, idx));
        }
    }
    flat.sort_by_key(|&(p, _)| p);

    let k = term_positions.len();
    let mut counts = vec![0usize; k];
    let mut have = 0usize;
    let mut left = 0usize;
    let mut best = (0usize, usize::MAX);

    for right in 0..flat.len() {
        let (_, ti) = flat[right];
        if counts[ti] == 0 {
            have += 1;
        }
        counts[ti] += 1;

        while have == k {
            let (lp, lti) = flat[left];
            let (rp, rti) = flat[right];
            let span = (rp + term_lens[rti]) - lp;
            if span < best.1 - best.0 || best.1 == usize::MAX {
                best = (lp, rp + term_lens[rti]);
            }
            counts[lti] -= 1;
            if counts[lti] == 0 {
                have -= 1;
            }
            left += 1;
        }
    }

    if best.1 == usize::MAX {
        let first = flat.first().map(|x| x.0).unwrap_or(0);
        (first, first)
    } else {
        best
    }
}

fn extract_fragment(content: &str, anchor: usize) -> String {
    let len = content.len();
    let start = floor_char_boundary(content, anchor.saturating_sub(FRAGMENT_RADIUS));
    let end = ceil_char_boundary(content, (anchor + FRAGMENT_RADIUS).min(len));
    let slice = &content[start..end];
    let normalized: String = slice.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < len { "..." } else { "" };
    format!("{prefix}{normalized}{suffix}")
}

/// Derive a short project tag from the Claude folder name encoding.
/// `-Users-recarnot-dev-agent-brain` -> `agent-brain`
/// `-Users-recarnot--claude` -> `claude`
/// `-Users-recarnot-.config-cc-interim-...` -> the tail
fn extract_project(path: &Path) -> String {
    let folder = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    for prefix in [
        "-Users-recarnot-dev-",
        "-Users-recarnot--",
        "-Users-recarnot-.",
        "-Users-recarnot-",
    ] {
        if let Some(rest) = folder.strip_prefix(prefix) {
            return rest.trim_matches('-').to_string();
        }
    }
    folder
}

/// Topic = frontmatter `description:` (preferred) or `name:`, else first H1, else "".
fn extract_topic(content: &str) -> String {
    let mut in_fm = false;
    let mut description: Option<String> = None;
    let mut name: Option<String> = None;
    let mut h1: Option<String> = None;

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if i == 0 && trimmed == "---" {
            in_fm = true;
            continue;
        }
        if in_fm {
            if trimmed == "---" {
                in_fm = false;
                continue;
            }
            if let Some(v) = trimmed.strip_prefix("description:") {
                description = Some(clean_yaml(v));
            } else if let Some(v) = trimmed.strip_prefix("name:") {
                name = Some(clean_yaml(v));
            }
        } else if h1.is_none() {
            if let Some(v) = trimmed.strip_prefix("# ") {
                h1 = Some(v.to_string());
            }
        }
    }

    description
        .or(name)
        .or(h1)
        .unwrap_or_default()
        .chars()
        .take(120)
        .collect()
}

fn clean_yaml(v: &str) -> String {
    v.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
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

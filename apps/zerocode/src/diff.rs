use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use similar::{ChangeTag, TextDiff};

const ADD_BG: Color = Color::Rgb(0, 40, 0);
const ADD_FG: Color = Color::Rgb(140, 240, 140);
const DEL_BG: Color = Color::Rgb(55, 0, 0);
const DEL_FG: Color = Color::Rgb(240, 140, 140);
const CTX_FG: Color = Color::Rgb(130, 130, 130);
const SEP_FG: Color = Color::Rgb(70, 70, 70);

const DIFF_CONTEXT: usize = 3;
const MAX_WRITE_LINES: usize = 60;

/// Build ratatui `Line`s for a unified diff of `old` vs `new`.
///
/// The `_lang` parameter is retained for API stability; per-language syntax
/// highlighting is no longer applied (the previous syntect-based highlighter
/// was removed to drop unmaintained transitive dependencies). Output is a
/// plain colored diff with +/-/context line backgrounds.
pub fn diff_lines(old: &str, new: &str, _lang: Option<&str>) -> Vec<Line<'static>> {
    let diff = TextDiff::from_lines(old, new);
    let mut out: Vec<Line<'static>> = Vec::new();

    for (gi, group) in diff.grouped_ops(DIFF_CONTEXT).iter().enumerate() {
        if gi > 0 {
            out.push(Line::from(Span::styled(
                "  \u{22ef}".to_string(),
                Style::default().fg(SEP_FG),
            )));
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let text = change.value().trim_end_matches('\n').to_string();
                let line = match change.tag() {
                    ChangeTag::Delete => {
                        let lineno = change
                            .old_index()
                            .map(|n| format!("{} | ", n + 1))
                            .unwrap_or_else(|| "  | ".to_string());
                        Line::from(vec![
                            Span::styled(
                                lineno + "- ",
                                Style::default()
                                    .bg(DEL_BG)
                                    .fg(DEL_FG)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(text, Style::default().bg(DEL_BG).fg(DEL_FG)),
                        ])
                        .style(Style::default().bg(DEL_BG))
                    }
                    ChangeTag::Insert => {
                        let lineno = change
                            .new_index()
                            .map(|n| format!("{} | ", n + 1))
                            .unwrap_or_else(|| "  | ".to_string());
                        Line::from(vec![
                            Span::styled(
                                lineno + "+ ",
                                Style::default()
                                    .bg(ADD_BG)
                                    .fg(ADD_FG)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(text, Style::default().bg(ADD_BG).fg(ADD_FG)),
                        ])
                        .style(Style::default().bg(ADD_BG))
                    }
                    ChangeTag::Equal => {
                        let lineno = change
                            .old_index()
                            .map(|n| format!("{} | ", n + 1))
                            .unwrap_or_else(|| "  | ".to_string());
                        Line::from(Span::styled(
                            format!("{}  {}", lineno, text),
                            Style::default().fg(CTX_FG),
                        ))
                    }
                };
                out.push(line);
            }
        }
    }

    if out.is_empty() {
        out.push(Line::from(Span::styled(
            "  (no changes)".to_string(),
            Style::default().fg(SEP_FG),
        )));
    }

    out
}

/// Build ratatui `Line`s showing `content` as entirely new (file_write).
///
/// The `_lang` parameter is retained for API stability; per-language syntax
/// highlighting is no longer applied. Capped at `MAX_WRITE_LINES`; a
/// `⋯ N more lines` trailer is appended when the file is larger.
pub fn write_lines(content: &str, _lang: Option<&str>) -> Vec<Line<'static>> {
    let all: Vec<&str> = content.lines().collect();
    let show = all.len().min(MAX_WRITE_LINES);

    let mut out: Vec<Line<'static>> = Vec::with_capacity(show + 1);

    for (i, item) in all.iter().enumerate().take(show) {
        out.push(
            Line::from(vec![
                Span::styled(
                    format!("{} | + ", i + 1),
                    Style::default()
                        .bg(ADD_BG)
                        .fg(ADD_FG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(item.to_string(), Style::default().bg(ADD_BG).fg(ADD_FG)),
            ])
            .style(Style::default().bg(ADD_BG)),
        );
    }

    if all.len() > MAX_WRITE_LINES {
        out.push(Line::from(Span::styled(
            format!("  \u{22ef} {} more lines", all.len() - MAX_WRITE_LINES),
            Style::default().fg(SEP_FG),
        )));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_produces_add_and_delete_lines() {
        let lines = diff_lines("foo\nbar\n", "foo\nbaz\n", None);
        let rendered: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert!(
            rendered
                .iter()
                .any(|s| s.contains("- ") && s.contains("bar"))
        );
        assert!(
            rendered
                .iter()
                .any(|s| s.contains("+ ") && s.contains("baz"))
        );
    }

    #[test]
    fn diff_no_changes_returns_placeholder() {
        let lines = diff_lines("same\n", "same\n", None);
        let all: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(all.contains("no changes"));
    }

    #[test]
    fn write_lines_caps_at_max() {
        let content: String = (0..100).map(|i| format!("line {i}\n")).collect();
        let lines = write_lines(&content, None);
        let last: String = lines
            .last()
            .unwrap()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(last.contains("more lines"), "expected trailer, got: {last}");
        assert_eq!(lines.len(), MAX_WRITE_LINES + 1);
    }

    #[test]
    fn diff_delete_line_has_red_bg() {
        let lines = diff_lines("old line\n", "new line\n", None);
        let del_line = lines
            .iter()
            .find(|l| {
                l.spans
                    .first()
                    .map(|s| s.content.as_ref().ends_with("- "))
                    .unwrap_or(false)
            })
            .expect("should have a delete line");
        assert_eq!(del_line.style.bg, Some(DEL_BG));
    }

    #[test]
    fn diff_insert_line_has_green_bg() {
        let lines = diff_lines("old line\n", "new line\n", None);
        let ins_line = lines
            .iter()
            .find(|l| {
                l.spans
                    .first()
                    .map(|s| s.content.as_ref().ends_with("+ "))
                    .unwrap_or(false)
            })
            .expect("should have an insert line");
        assert_eq!(ins_line.style.bg, Some(ADD_BG));
    }

    #[test]
    fn test_diff_lines_shows_left_aligned_line_numbers() {
        let old = "line one\nline two\n";
        let new = "line one\nline three\n";
        let lines = diff_lines(old, new, None);
        let first = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("three")))
            .unwrap();
        assert!(
            first.spans[0]
                .content
                .starts_with(|c: char| c.is_ascii_digit()),
            "expected left-aligned line number"
        );

        let write_lines = write_lines("first\nsecond\nthird", None);
        assert!(write_lines[0].spans[0].content.starts_with("1 | + "));
    }
}

use std::fmt::Write;

use crate::parser::node::Span;

/// How severe is a diagnostic?
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Severity::Error => RED,
            Severity::Warning => YELLOW,
            Severity::Info => BLUE,
        }
    }
}

/// A problem to report to the human.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub message: String,
}

// ANSI escape codes used to brighten up terminal output.
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";

impl Diagnostic {
    pub fn error(at: impl Into<Span>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            span: at.into(),
            message: message.into(),
        }
    }

    /// Render this diagnostic as a multi-line, human-friendly report pointing
    /// at the offending span in `source`.
    ///
    /// Spans are char offsets (the lexer scans a `Vec<char>`), so all the
    /// position math here counts chars rather than bytes.
    ///
    /// Pass `color: true` to include ANSI color escapes.
    pub fn render(&self, source: &str, color: bool) -> String {
        let paint = |text: &str, code: &str| {
            if color {
                format!("{code}{text}{RESET}")
            } else {
                text.to_owned()
            }
        };

        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let start = self.span.start.min(len);
        let end = self.span.end.clamp(start, len);

        // Find the line containing `start`: its 1-based number and the char
        // offset where it begins.
        let mut line_no = 1usize;
        let mut line_start = 0usize;
        for (i, c) in chars.iter().enumerate().take(start) {
            if *c == '\n' {
                line_no += 1;
                line_start = i + 1;
            }
        }

        // Extend to the end of that line.
        let mut line_end = line_start;
        while line_end < len && chars[line_end] != '\n' {
            line_end += 1;
        }

        let line_text: String = chars[line_start..line_end].iter().collect();
        let column = start - line_start; // 0-based column within the line

        // Underline the span, confined to this line, with at least one caret.
        let underline_end = end.min(line_end);
        let width = underline_end.saturating_sub(start).max(1);

        let sev = self.severity;
        let gutter = line_no.to_string();
        let pad = " ".repeat(gutter.len());
        let bar = paint("│", BLUE);
        let offset = " ".repeat(column);
        let carets = paint(&"^".repeat(width), sev.color());
        let connector = paint("╰─", sev.color());

        let mut out = String::new();
        // Header: "error: <message>"
        let header = if color {
            format!(
                "{BOLD}{}{}{RESET}: {}",
                sev.color(),
                sev.label(),
                self.message
            )
        } else {
            format!("{}: {}", sev.label(), self.message)
        };
        let _ = writeln!(out, "{header}");
        // Location: " --> line L:C"
        let _ = writeln!(
            out,
            "{} {} {}",
            pad,
            paint("-->", BLUE),
            paint(&format!("line {}:{}", line_no, column + 1), DIM),
        );
        // Blank gutter line.
        let _ = writeln!(out, "{} {}", pad, bar);
        // The offending source line.
        let _ = writeln!(out, "{} {} {}", paint(&gutter, BLUE), bar, line_text);
        // The squigglies.
        let _ = writeln!(out, "{} {} {}{}", pad, bar, offset, carets);
        // The description, hanging off an arrow under the squigglies.
        let _ = write!(
            out,
            "{} {} {}{} {}",
            pad,
            bar,
            offset,
            connector,
            paint(&self.message, sev.color()),
        );
        out
    }
}

pub fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_line_and_squigglies() {
        // span covers the second `a` (the initializer) on line 3.
        let source = "var x = 1;\nvar y = 2;\nvar a = a;";
        // char offset of the final `a`:
        let start = source.char_indices().rev().nth(1).unwrap().0;
        let span = Span {
            start,
            end: start + 1,
        };
        let diag = Diagnostic::error(span, "Can't read local variable in its own initializer.");

        let out = diag.render(source, false);
        let lines: Vec<&str> = out.lines().collect();

        // Header carries the severity and message.
        assert_eq!(
            lines[0],
            "error: Can't read local variable in its own initializer."
        );
        // Location reports line 3, column 9.
        assert_eq!(lines[1], "  --> line 3:9");
        // The offending source line is shown with its line number.
        assert_eq!(lines[3], "3 │ var a = a;");
        // The caret sits under the offending char (8 spaces of offset).
        assert_eq!(lines[4], "  │         ^");
        // The description hangs off an arrow aligned with the caret.
        assert_eq!(
            lines[5],
            "  │         ╰─ Can't read local variable in its own initializer."
        );
    }

    #[test]
    fn multi_char_span_widens_underline() {
        let source = "print foobar;";
        let start = 6; // "foobar"
        let span = Span { start, end: 12 };
        let diag = Diagnostic::error(span, "nope");

        let out = diag.render(source, false);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[4], "  │       ^^^^^^");
    }

    #[test]
    fn zero_width_span_still_shows_one_caret() {
        let source = "1 +";
        let span = Span { start: 3, end: 3 };
        let diag = Diagnostic::error(span, "expected an expression");
        let out = diag.render(source, false);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[4], "  │    ^");
    }
}

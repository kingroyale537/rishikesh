use crate::source::{SourceFile, Span};
use colored::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Note,
}

impl DiagnosticLevel {
    pub fn name(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Info => "info",
            DiagnosticLevel::Note => "note",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<Span>,
    pub label: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span: None,
            label: None,
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span: None,
            label: None,
            help: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn render(&self, source: &SourceFile) -> String {
        let mut out = String::new();
        let level_str = match self.level {
            DiagnosticLevel::Error => "error".bold().red(),
            DiagnosticLevel::Warning => "warning".bold().yellow(),
            DiagnosticLevel::Info => "info".bold().cyan(),
            DiagnosticLevel::Note => "note".bold().green(),
        };

        if let Some(span) = self.span {
            let loc = source.location(span.start);
            out.push_str(&format!("{}: {}\n", level_str, self.message.bold()));
            out.push_str(&format!(
                "  {} {}:{}:{}\n",
                "-->".blue().bold(),
                source.name,
                loc.line,
                loc.column
            ));

            if let Some(line_str) = source.get_line(loc.line) {
                let line_num_str = format!("{}", loc.line);
                let padding = " ".repeat(line_num_str.len());
                out.push_str(&format!("   {} |\n", padding));
                out.push_str(&format!(" {} | {}\n", line_num_str.blue().bold(), line_str));

                let col_offset = loc.column.saturating_sub(1);
                let underline_len = (span.len()).max(1).min(line_str.len().saturating_sub(col_offset).max(1));
                let underline = match self.level {
                    DiagnosticLevel::Error => "^".repeat(underline_len).red().bold(),
                    DiagnosticLevel::Warning => "^".repeat(underline_len).yellow().bold(),
                    DiagnosticLevel::Info => "^".repeat(underline_len).cyan().bold(),
                    DiagnosticLevel::Note => "^".repeat(underline_len).green().bold(),
                };

                let label_text = self.label.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "   {} | {}{} {}\n",
                    padding,
                    " ".repeat(col_offset),
                    underline,
                    label_text.red()
                ));
                out.push_str(&format!("   {} |\n", padding));
            }
        } else {
            out.push_str(&format!("{}: {}\n", level_str, self.message.bold()));
        }

        if let Some(help) = &self.help {
            out.push_str(&format!(
                "   = {}: {}\n",
                "help".bold().cyan(),
                help
            ));
        }

        out
    }
}

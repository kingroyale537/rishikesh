pub mod diagnostic;
pub mod source;

pub use diagnostic::{Diagnostic, DiagnosticLevel};
pub use source::{Location, SourceFile, Span};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_mapping() {
        let src = SourceFile::new("test.rk", "let x = 10\nlet y = 20\nlet z = 30");
        let loc1 = src.location(0);
        assert_eq!(loc1.line, 1);
        assert_eq!(loc1.column, 1);

        let loc2 = src.location(11); // start of line 2
        assert_eq!(loc2.line, 2);
        assert_eq!(loc2.column, 1);

        let line2 = src.get_line(2);
        assert_eq!(line2, Some("let y = 20"));
    }
}

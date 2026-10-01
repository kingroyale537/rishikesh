#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn point(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,   // 1-indexed
    pub column: usize, // 1-indexed
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub content: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let mut line_starts = vec![0];
        for (i, b) in content.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            name: name.into(),
            content,
            line_starts,
        }
    }

    pub fn location(&self, offset: usize) -> Location {
        let offset = offset.min(self.content.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line_idx) => line_idx,
            Err(line_idx) => line_idx - 1,
        };
        let line_start = self.line_starts[line];
        let col = offset - line_start;
        Location {
            line: line + 1,
            column: col + 1,
        }
    }

    pub fn get_line(&self, line_1_indexed: usize) -> Option<&str> {
        if line_1_indexed == 0 || line_1_indexed > self.line_starts.len() {
            return None;
        }
        let line_idx = line_1_indexed - 1;
        let start = self.line_starts[line_idx];
        let end = if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1].saturating_sub(1)
        } else {
            self.content.len()
        };
        Some(&self.content[start..end])
    }
}

use crate::token::{Token, TokenKind};
use rishi_span::Span;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum LexerError {
    #[error("Unexpected character `{0}` at offset {1}")]
    UnexpectedChar(char, usize),
    #[error("Unterminated string literal starting at offset {0}")]
    UnterminatedString(usize),
    #[error("Invalid indentation level (inconsistent with stack) at offset {0}")]
    InconsistentDedent(usize),
    #[error("Invalid number literal at offset {0}")]
    InvalidNumber(usize),
}

pub struct Lexer<'a> {
    src: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    indent_stack: Vec<usize>,
    paren_depth: usize,
    at_line_start: bool,
    pending_tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let chars: Vec<(usize, char)> = src.char_indices().collect();
        Self {
            src,
            chars,
            cursor: 0,
            indent_stack: vec![0],
            paren_depth: 0,
            at_line_start: true,
            pending_tokens: Vec::new(),
        }
    }

    pub fn tokenize_all(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        if let Some(tok) = self.pending_tokens.pop() {
            return Ok(tok);
        }

        if self.at_line_start && self.paren_depth == 0 {
            self.handle_indentation()?;
            if let Some(tok) = self.pending_tokens.pop() {
                return Ok(tok);
            }
        }

        self.skip_whitespace_and_comments();

        let start_pos = self.current_pos();
        if self.is_at_end() {
            // Emit remaining dedents
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                return Ok(Token::new(TokenKind::Dedent, Span::new(start_pos, start_pos)));
            }
            return Ok(Token::new(TokenKind::Eof, Span::new(start_pos, start_pos)));
        }

        let ch = self.peek();

        // Newlines
        if ch == '\n' || ch == '\r' {
            self.advance();
            if ch == '\r' && self.peek() == '\n' {
                self.advance();
            }
            if self.paren_depth == 0 {
                self.at_line_start = true;
                return Ok(Token::new(TokenKind::Newline, Span::new(start_pos, self.current_pos())));
            } else {
                return self.next_token();
            }
        }

        // Identifiers & Keywords
        if is_ident_start(ch) {
            return Ok(self.lex_identifier(start_pos));
        }

        // Numbers
        if ch.is_ascii_digit() {
            return self.lex_number(start_pos);
        }

        // Strings
        if ch == '"' || ch == '\'' {
            return self.lex_string(start_pos, ch);
        }

        // Symbols and Operators
        self.advance();
        let tok_kind = match ch {
            '(' => {
                self.paren_depth += 1;
                TokenKind::LParen
            }
            ')' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RParen
            }
            '[' => {
                self.paren_depth += 1;
                TokenKind::LBracket
            }
            ']' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RBracket
            }
            '{' => {
                self.paren_depth += 1;
                TokenKind::LBrace
            }
            '}' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RBrace
            }
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '+' => {
                if self.match_char('=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.match_char('>') {
                    TokenKind::Arrow
                } else if self.match_char('=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.match_char('*') {
                    TokenKind::Power
                } else if self.match_char('=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.match_char('=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            '%' => TokenKind::Percent,
            '=' => {
                if self.match_char('=') {
                    TokenKind::EqEq
                } else if self.match_char('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.match_char('=') {
                    TokenKind::NotEq
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                if self.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.match_char('&') {
                    TokenKind::And
                } else {
                    return Err(LexerError::UnexpectedChar('&', start_pos));
                }
            }
            '|' => {
                if self.match_char('>') {
                    TokenKind::Pipeline
                } else if self.match_char('|') {
                    TokenKind::Or
                } else {
                    return Err(LexerError::UnexpectedChar('|', start_pos));
                }
            }
            '?' => {
                if self.match_char('?') {
                    TokenKind::NullCoalesce
                } else if self.match_char('.') {
                    TokenKind::SafeDot
                } else {
                    TokenKind::Question
                }
            }
            '.' => {
                if self.match_char('.') {
                    if self.match_char('=') {
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            other => return Err(LexerError::UnexpectedChar(other, start_pos)),
        };

        Ok(Token::new(tok_kind, Span::new(start_pos, self.current_pos())))
    }

    fn handle_indentation(&mut self) -> Result<(), LexerError> {
        self.at_line_start = false;
        let mut indent_spaces = 0;
        let line_start_pos = self.current_pos();

        while !self.is_at_end() {
            let ch = self.peek();
            if ch == ' ' {
                indent_spaces += 1;
                self.advance();
            } else if ch == '\t' {
                indent_spaces += 4; // Treat tab as 4 spaces
                self.advance();
            } else if ch == '#' {
                // Ignore comment line
                self.skip_line_comment();
                if self.peek() == '\n' || self.peek() == '\r' {
                    self.advance();
                    if self.peek() == '\n' {
                        self.advance();
                    }
                    indent_spaces = 0;
                }
            } else if ch == '\n' || ch == '\r' {
                // Blank line, reset indent count
                self.advance();
                if ch == '\r' && self.peek() == '\n' {
                    self.advance();
                }
                indent_spaces = 0;
            } else {
                break;
            }
        }

        if self.is_at_end() {
            return Ok(());
        }

        let current_indent = *self.indent_stack.last().unwrap_or(&0);
        if indent_spaces > current_indent {
            self.indent_stack.push(indent_spaces);
            self.pending_tokens.push(Token::new(
                TokenKind::Indent,
                Span::new(line_start_pos, self.current_pos()),
            ));
        } else if indent_spaces < current_indent {
            let mut dedents = Vec::new();
            while let Some(&top) = self.indent_stack.last() {
                if top == indent_spaces {
                    break;
                }
                if top < indent_spaces {
                    return Err(LexerError::InconsistentDedent(line_start_pos));
                }
                self.indent_stack.pop();
                dedents.push(Token::new(
                    TokenKind::Dedent,
                    Span::new(line_start_pos, self.current_pos()),
                ));
            }
            // Reverse so they pop in correct order
            dedents.reverse();
            self.pending_tokens.extend(dedents);
        }

        Ok(())
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let ch = self.peek();
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else if ch == '#' {
                self.skip_line_comment();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' && self.peek() != '\r' {
            self.advance();
        }
    }

    fn lex_identifier(&mut self, start: usize) -> Token {
        while !self.is_at_end() && is_ident_continue(self.peek()) {
            self.advance();
        }
        let end = self.current_pos();
        let text = &self.src[start..end];

        let kind = match text {
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "match" => TokenKind::Match,
            "struct" => TokenKind::Struct,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "enum" => TokenKind::Enum,
            "component" => TokenKind::Component,
            "spawn" => TokenKind::Spawn,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "const" => TokenKind::Const,
            "self" => TokenKind::SelfKw,
            "True" | "true" => TokenKind::Bool(true),
            "False" | "false" => TokenKind::Bool(false),
            "None" | "nil" | "null" => TokenKind::None,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Ident(text.to_string()),
        };

        Token::new(kind, Span::new(start, end))
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, LexerError> {
        let mut is_float = false;
        if self.peek() == '0' && (self.peek_next() == 'x' || self.peek_next() == 'X') {
            self.advance(); // '0'
            self.advance(); // 'x'
            while !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                self.advance();
            }
            let text = &self.src[start + 2..self.current_pos()];
            let val = i64::from_str_radix(text, 16).map_err(|_| LexerError::InvalidNumber(start))?;
            return Ok(Token::new(TokenKind::Int(val), Span::new(start, self.current_pos())));
        }

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            is_float = true;
            self.advance(); // '.'
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                self.advance();
            }
        }

        // Scientific notation: 1e-4, 2E+10
        if self.peek() == 'e' || self.peek() == 'E' {
            is_float = true;
            self.advance();
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                self.advance();
            }
        }

        let raw = &self.src[start..self.current_pos()];
        let clean: String = raw.chars().filter(|&c| c != '_').collect();

        if is_float {
            let val: f64 = clean.parse().map_err(|_| LexerError::InvalidNumber(start))?;
            Ok(Token::new(TokenKind::Float(val), Span::new(start, self.current_pos())))
        } else {
            let val: i64 = clean.parse().map_err(|_| LexerError::InvalidNumber(start))?;
            Ok(Token::new(TokenKind::Int(val), Span::new(start, self.current_pos())))
        }
    }

    fn lex_string(&mut self, start: usize, quote: char) -> Result<Token, LexerError> {
        let is_triple = self.peek_next() == quote && self.peek_nth(2) == quote;
        if is_triple {
            self.advance(); // quote 1
            self.advance(); // quote 2
            self.advance(); // quote 3
        } else {
            self.advance(); // single quote
        }

        let mut content = String::new();
        while !self.is_at_end() {
            if is_triple {
                if self.peek() == quote && self.peek_next() == quote && self.peek_nth(2) == quote {
                    self.advance();
                    self.advance();
                    self.advance();
                    return Ok(Token::new(TokenKind::String(content), Span::new(start, self.current_pos())));
                }
            } else if self.peek() == quote {
                self.advance();
                return Ok(Token::new(TokenKind::String(content), Span::new(start, self.current_pos())));
            }

            let ch = self.peek();
            if ch == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(LexerError::UnterminatedString(start));
                }
                let esc = self.peek();
                self.advance();
                match esc {
                    'n' => content.push('\n'),
                    't' => content.push('\t'),
                    'r' => content.push('\r'),
                    '\\' => content.push('\\'),
                    '\'' => content.push('\''),
                    '"' => content.push('"'),
                    other => {
                        content.push('\\');
                        content.push(other);
                    }
                }
            } else {
                content.push(ch);
                self.advance();
            }
        }

        Err(LexerError::UnterminatedString(start))
    }

    fn peek(&self) -> char {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor].1
        } else {
            '\0'
        }
    }

    fn peek_next(&self) -> char {
        self.peek_nth(1)
    }

    fn peek_nth(&self, n: usize) -> char {
        if self.cursor + n < self.chars.len() {
            self.chars[self.cursor + n].1
        } else {
            '\0'
        }
    }

    fn advance(&mut self) -> char {
        if self.cursor < self.chars.len() {
            let ch = self.chars[self.cursor].1;
            self.cursor += 1;
            ch
        } else {
            '\0'
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.chars.len()
    }

    fn current_pos(&self) -> usize {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor].0
        } else {
            self.src.len()
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

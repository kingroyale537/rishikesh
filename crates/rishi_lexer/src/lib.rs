pub mod lexer;
pub mod token;

pub use lexer::{Lexer, LexerError};
pub use token::{Token, TokenKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let src = "let x = 42\nmut y: Float = 3.14\n";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Int(42),
                TokenKind::Newline,
                TokenKind::Mut,
                TokenKind::Ident("y".to_string()),
                TokenKind::Colon,
                TokenKind::Ident("Float".to_string()),
                TokenKind::Eq,
                TokenKind::Float(3.14),
                TokenKind::Newline,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_indentation_block() {
        let src = "fn foo():\n    let a = 1\n    let b = 2\nlet c = 3\n";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Fn,
                TokenKind::Ident("foo".to_string()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Let,
                TokenKind::Ident("a".to_string()),
                TokenKind::Eq,
                TokenKind::Int(1),
                TokenKind::Newline,
                TokenKind::Let,
                TokenKind::Ident("b".to_string()),
                TokenKind::Eq,
                TokenKind::Int(2),
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Let,
                TokenKind::Ident("c".to_string()),
                TokenKind::Eq,
                TokenKind::Int(3),
                TokenKind::Newline,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_operators_and_pipeline() {
        let src = "x |> filter(fn(x): x > 0) ?? \"default\"";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();

        assert!(kinds.contains(&TokenKind::Pipeline));
        assert!(kinds.contains(&TokenKind::NullCoalesce));
    }
}

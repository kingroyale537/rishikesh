pub mod parser;

pub use parser::{ParseError, Parser};

#[cfg(test)]
mod tests {
    use super::*;
    use rishi_ast::*;
    use rishi_lexer::Lexer;

    fn parse_src(src: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_parse_let_and_binary() {
        let src = "let x = 10 + 20 * 3\n";
        let prog = parse_src(src).unwrap();
        assert_eq!(prog.statements.len(), 1);

        match &prog.statements[0].kind {
            StmtKind::Let { name, init, .. } => {
                assert_eq!(name, "x");
                // Precedence: 10 + (20 * 3)
                if let ExprKind::Binary { op, right, .. } = &init.kind {
                    assert_eq!(*op, BinOp::Add);
                    if let ExprKind::Binary { op: r_op, .. } = &right.kind {
                        assert_eq!(*r_op, BinOp::Mul);
                    } else {
                        panic!("Expected right operand to be binary mul");
                    }
                } else {
                    panic!("Expected binary add");
                }
            }
            _ => panic!("Expected Let statement"),
        }
    }

    #[test]
    fn test_parse_fn_decl() {
        let src = "fn add(a: Int, b: Int) -> Int:\n    return a + b\n";
        let prog = parse_src(src).unwrap();
        assert_eq!(prog.statements.len(), 1);

        match &prog.statements[0].kind {
            StmtKind::FnDecl { name, params, body, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected FnDecl statement"),
        }
    }

    #[test]
    fn test_parse_pipeline() {
        let src = "let result = [1, 2, 3] |> sum()\n";
        let prog = parse_src(src).unwrap();
        assert_eq!(prog.statements.len(), 1);
    }
}

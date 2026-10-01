pub mod builtins;
pub mod env;
pub mod evaluator;
pub mod value;

pub use builtins::register_builtins;
pub use env::Environment;
pub use evaluator::{ControlFlow, Evaluator};
pub use value::{EvalError, Value};

#[cfg(test)]
mod tests {
    use super::*;
    use rishi_lexer::Lexer;
    use rishi_parser::Parser;

    fn eval_code(src: &str) -> Result<Value, EvalError> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();
        let mut evaluator = Evaluator::new();
        evaluator.eval_program(&prog)
    }

    #[test]
    fn test_arithmetic_and_variables() {
        let src = r#"
let a = 10
let b = 20
let c = a + b * 2
assert_eq(c, 50)
"#;
        eval_code(src).unwrap();
    }

    #[test]
    fn test_mutability_check() {
        let src_err = r#"
let a = 10
a = 20
"#;
        assert!(eval_code(src_err).is_err());

        let src_ok = r#"
mut a = 10
a = 20
assert_eq(a, 20)
"#;
        eval_code(src_ok).unwrap();
    }

    #[test]
    fn test_functions_and_recursion() {
        let src = r#"
fn factorial(n):
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)

let res = factorial(5)
assert_eq(res, 120)
"#;
        eval_code(src).unwrap();
    }

    #[test]
    fn test_pipelines_and_lambdas() {
        let src = r#"
let numbers = [1, 2, 3, 4, 5]
let doubled = numbers |> map((x) => x * 2)
let total = doubled |> sum()
assert_eq(total, 30)
"#;
        eval_code(src).unwrap();
    }

    #[test]
    fn test_structs_and_methods() {
        let src = r#"
struct Point:
    x: Int
    y: Int

    fn sum(self):
        return self.x + self.y

let p = Point(10, 25)
assert_eq(p.x, 10)
assert_eq(p.y, 25)
assert_eq(p.sum(), 35)
"#;
        eval_code(src).unwrap();
    }

    #[test]
    fn test_string_interpolation() {
        let src = r#"
let name = "Rishikesh"
let age = 21
let msg = "Hello {name}, in 5 years you will be {age + 5}!"
assert_eq(msg, "Hello Rishikesh, in 5 years you will be 26!")
"#;
        eval_code(src).unwrap();
    }

    #[test]
    fn test_loops() {
        let src = r#"
mut sum_val = 0
for i in range(1, 6):
    sum_val += i
assert_eq(sum_val, 15)
"#;
        eval_code(src).unwrap();
    }
}

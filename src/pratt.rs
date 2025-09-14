use crate::pratt::ExpressionError::{
    DidntGetRightParen, RanOutOfTokens, UnboundVariable, UnexpectedToken, UnsupportedOperation,
};
use core::fmt;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Eq, Debug, PartialEq)]
pub enum ExpressionError {
    UnexpectedToken(Token),
    UnboundVariable(String),
    UnsupportedOperation,
    RanOutOfTokens,
    DidntGetRightParen,
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnexpectedToken(t) => write!(f, "Unexpected token: {t:?}"),
            UnboundVariable(v) => write!(f, "Unbound variable: {v}"),
            UnsupportedOperation => write!(f, "Unsupported operation"),
            RanOutOfTokens => write!(f, "Ran out of tokens"),
            DidntGetRightParen => write!(f, "Didn't get right paren"),
        }
    }
}

#[derive(Eq, Debug, PartialEq, Clone)]
pub enum Token {
    Plus,
    Minus,
    Multiply,
    Equals,
    Num(i32),
    Var(String),
    Str(String),
    Bool(bool),
    ParenOpen,
    ParenClose,
    NotEquals,
    Not,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Multiply => write!(f, "*"),
            Token::Equals => write!(f, "="),
            Token::Num(n) => write!(f, "{n}"),
            Token::Var(v) => write!(f, "{v}"),
            Token::Str(s) => write!(f, "{s}"),
            Token::Bool(b) => write!(f, "{b}"),
            Token::ParenOpen => write!(f, "("),
            Token::ParenClose => write!(f, ")"),
            Token::NotEquals => write!(f, "!="),
            Token::Not => write!(f, "!"),
        }
    }
}

#[derive(Clone)]
pub enum ASTNode {
    Num(i32),
    Str(String),
    Var(String),
    Bool(bool),
    Add(Box<ASTNode>, Box<ASTNode>),
    Sub(Box<ASTNode>, Box<ASTNode>),
    Mul(Box<ASTNode>, Box<ASTNode>),
    Eq(Box<ASTNode>, Box<ASTNode>),
    Not(Box<ASTNode>),
    NotEq(Box<ASTNode>, Box<ASTNode>),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn parse(&mut self, precedence: i32) -> Result<ASTNode, ExpressionError> {
        let mut result = match self.get_next_token() {
            Some(Token::Minus) if precedence < 30 => {
                self.consume_operator();
                Ok(ASTNode::Sub(
                    Box::new(ASTNode::Num(0)),
                    Box::new(self.parse(30)?),
                ))
            }
            Some(Token::Not) if precedence < 40 => {
                self.consume_operator();
                Ok(ASTNode::Not(Box::new(self.parse(40)?)))
            }
            Some(Token::Num(val)) => {
                let r = ASTNode::Num(*val);
                self.consume_operator();
                Ok(r)
            }
            Some(Token::Var(val)) => {
                let r = ASTNode::Var(val.clone());
                self.consume_operator();
                Ok(r)
            }
            Some(Token::Str(val)) => {
                let r = ASTNode::Str(val.clone());
                self.consume_operator();
                Ok(r)
            }
            Some(Token::Bool(val)) => {
                let r = ASTNode::Bool(*val);
                self.consume_operator();
                Ok(r)
            }
            Some(Token::ParenOpen) if precedence < 50 => {
                self.consume_operator();
                let subexpr = self.parse(0)?;
                self.expect_paren_close()?;
                Ok(subexpr)
            }
            Some(t) => Err(ExpressionError::UnexpectedToken(t.clone())),
            None => Err(ExpressionError::RanOutOfTokens),
        }?;

        loop {
            match self.get_next_token() {
                Some(Token::Plus) if precedence < 20 => {
                    self.consume_operator();
                    result = ASTNode::Add(Box::new(result), Box::new(self.parse(20)?));
                }
                Some(Token::Minus) if precedence < 20 => {
                    self.consume_operator();
                    result = ASTNode::Sub(Box::new(result), Box::new(self.parse(20)?));
                }
                Some(Token::Multiply) if precedence < 30 => {
                    self.consume_operator();
                    result = ASTNode::Mul(Box::new(result), Box::new(self.parse(30)?));
                }
                Some(Token::Equals) if precedence < 10 => {
                    self.consume_operator();
                    result = ASTNode::Eq(Box::new(result), Box::new(self.parse(10)?));
                }
                Some(Token::NotEquals) if precedence < 10 => {
                    self.consume_operator();
                    result = ASTNode::NotEq(Box::new(result), Box::new(self.parse(10)?));
                }
                _ => break,
            }
        }

        Ok(result)
    }

    fn consume_operator(&mut self) {
        self.pos += 1;
    }

    fn expect_paren_close(&mut self) -> Result<(), ExpressionError> {
        match self.get_next_token() {
            Some(Token::ParenClose) => {
                self.consume_operator();
                Ok(())
            }
            _ => Err(DidntGetRightParen),
        }
    }

    fn get_next_token(&self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            Some(&self.tokens[self.pos])
        } else {
            None
        }
    }
}

#[derive(Eq, Debug, PartialEq, Clone)]
pub enum Value {
    Str(String),
    Num(i32),
    Bool(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Str(s) => write!(f, "{s}"),
            Value::Num(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
        }
    }
}

pub fn eval(ast: ASTNode, environment: &HashMap<String, Value>) -> Result<Value, ExpressionError> {
    match ast {
        ASTNode::Num(num) => Ok(Value::Num(num)),
        ASTNode::Str(s) => Ok(Value::Str(s)),
        ASTNode::Bool(b) => Ok(Value::Bool(b)),
        ASTNode::Var(name) => match environment.get(&name) {
            Some(val) => Ok(val.clone()),
            None => Err(ExpressionError::UnboundVariable(name)),
        },
        ASTNode::Add(lhs, rhs) => {
            let lhs = eval(*lhs, environment)?;
            let rhs = eval(*rhs, environment)?;
            match (lhs, rhs) {
                (Value::Num(l), Value::Num(r)) => Ok(Value::Num(l + r)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
        ASTNode::Sub(lhs, rhs) => {
            let lhs = eval(*lhs, environment)?;
            let rhs = eval(*rhs, environment)?;
            match (lhs, rhs) {
                (Value::Num(l), Value::Num(r)) => Ok(Value::Num(l - r)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
        ASTNode::Mul(lhs, rhs) => {
            let lhs = eval(*lhs, environment)?;
            let rhs = eval(*rhs, environment)?;
            match (lhs, rhs) {
                (Value::Num(l), Value::Num(r)) => Ok(Value::Num(l * r)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
        ASTNode::Eq(lhs, rhs) => {
            let lhs = eval(*lhs, environment)?;
            let rhs = eval(*rhs, environment)?;
            match (lhs, rhs) {
                (Value::Num(l), Value::Num(r)) => Ok(Value::Bool(l == r)),
                (Value::Str(l), Value::Str(r)) => Ok(Value::Bool(l == r)),
                (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l == r)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
        ASTNode::NotEq(lhs, rhs) => {
            let lhs = eval(*lhs, environment)?;
            let rhs = eval(*rhs, environment)?;
            match (lhs, rhs) {
                (Value::Num(l), Value::Num(r)) => Ok(Value::Bool(l != r)),
                (Value::Str(l), Value::Str(r)) => Ok(Value::Bool(l != r)),
                (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
        ASTNode::Not(child) => {
            let val = eval(*child, environment)?;
            match val {
                Value::Bool(n) => Ok(Value::Bool(!n)),
                _ => Err(ExpressionError::UnsupportedOperation),
            }
        }
    }
}

pub fn lex(input: &str) -> Result<Vec<Token>, ExpressionError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '-' => {
                tokens.push(Token::Minus);
                chars.next();
            }
            '*' => {
                tokens.push(Token::Multiply);
                chars.next();
            }
            '=' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Equals);
                } else {
                    return Err(ExpressionError::UnexpectedToken(Token::Var(c.to_string())));
                }
            }
            '!' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::NotEquals);
                } else {
                    tokens.push(Token::Not);
                }
            }
            '(' => {
                tokens.push(Token::ParenOpen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::ParenClose);
                chars.next();
            }
            ' ' => {
                chars.next();
            } // Ignore spaces
            '0'..='9' => {
                let mut num = String::new();
                while let Some('0'..='9') = chars.peek() {
                    num.push(chars.next().unwrap());
                }
                let num = i32::from_str(&num).unwrap(); // This unwrap can be replaced by better error handling
                tokens.push(Token::Num(num));
            }
            '\"' => {
                let mut str = String::new();
                chars.next(); // Skip the initial quote
                while let Some(&c) = chars.peek() {
                    if c == '\"' {
                        chars.next();
                        break;
                    } else {
                        str.push(chars.next().unwrap());
                    }
                }
                tokens.push(Token::Str(str));
            }
            _ => {
                if c.is_alphabetic() {
                    let mut var = String::new();
                    while let Some(c) = chars.peek() {
                        if !c.is_alphanumeric() {
                            break;
                        }
                        var.push(chars.next().unwrap());
                    }
                    tokens.push(Token::Var(var));
                } else {
                    return Err(ExpressionError::UnexpectedToken(Token::Var(c.to_string())));
                }
            }
        }
    }

    Ok(tokens)
}

pub fn lex_parse(input: String) -> Result<ASTNode, ExpressionError> {
    let tokens = lex(&input)?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_str_eq() {
        let ast = lex_parse("\"hello\" == \"hello\"".to_string()).unwrap();
        let mut environment = HashMap::<String, Value>::new();
        assert_eq!(eval(ast, &mut environment).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eval_str_eq_not() {
        let ast = lex_parse("\"hello\" == \"work\"".to_string()).unwrap();
        let mut environment = HashMap::<String, Value>::new();
        assert_eq!(eval(ast, &mut environment).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_eval_str_ne() {
        let ast = lex_parse("\"hello\" != \"world\"".to_string()).unwrap();
        let mut environment = HashMap::<String, Value>::new();
        assert_eq!(eval(ast, &mut environment).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eval_parens() {
        let ast = lex_parse("(1 + 2) * 3 == ((9))".to_string()).unwrap();
        let mut environment = HashMap::<String, Value>::new();
        assert_eq!(eval(ast, &mut environment).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_lex() {
        assert_eq!(
            lex("1 + 2 * 3").unwrap(),
            vec![
                Token::Num(1),
                Token::Plus,
                Token::Num(2),
                Token::Multiply,
                Token::Num(3),
            ]
        );

        assert_eq!(
            lex("\"hello\" == \"world\"").unwrap(),
            vec![
                Token::Str("hello".to_string()),
                Token::Equals,
                Token::Str("world".to_string()),
            ]
        );

        assert_eq!(
            lex("(1 - 2)").unwrap(),
            vec![
                Token::ParenOpen,
                Token::Num(1),
                Token::Minus,
                Token::Num(2),
                Token::ParenClose,
            ]
        );
    }

    #[test]
    fn test_parse_math() {
        let mut parser = Parser {
            tokens: vec![
                Token::Num(1),
                Token::Plus,
                Token::Num(2),
                Token::Multiply,
                Token::Num(3),
            ],
            pos: 0,
        };

        let ast = parser.parse(0).unwrap();
        assert_eq!(
            eval(ast, &HashMap::<String, Value>::new()).unwrap(),
            Value::Num(7) // due to operator precedence, this is 1 + (2 * 3) = 7
        );
    }

    #[test]
    fn test_parse_minus() {
        let mut parser = Parser {
            tokens: lex("110 - 100").unwrap(),
            pos: 0,
        };

        let ast = parser.parse(0).unwrap();
        assert_eq!(
            eval(ast, &HashMap::<String, Value>::new()).unwrap(),
            Value::Num(10)
        );
    }

    #[test]
    fn test_parse_string_eq() {
        let mut parser = Parser {
            tokens: vec![
                Token::Str("hello".to_string()),
                Token::Equals,
                Token::Str("world".to_string()),
            ],
            pos: 0,
        };
        let ast = parser.parse(0).unwrap();
        assert_eq!(
            eval(ast, &HashMap::<String, Value>::new()).unwrap(),
            Value::Bool(false) // "hello" != "world"
        );
    }

    #[test]
    fn test_parse_string_neq() {
        let mut parser = Parser {
            tokens: vec![
                Token::ParenOpen,
                Token::Num(1),
                Token::Minus,
                Token::Num(2),
                Token::ParenClose,
            ],
            pos: 0,
        };
        let ast = parser.parse(0).unwrap();
        assert_eq!(
            eval(ast, &HashMap::<String, Value>::new()).unwrap(),
            Value::Num(-1) // (1 - 2) = -1
        );
    }

    #[test]
    fn test_eval_with_environment() {
        let mut environment = HashMap::new();
        environment.insert("x".to_string(), Value::Num(5));
        environment.insert("y".to_string(), Value::Num(10));

        // Test evaluation of variables in the environment
        let ast_x = ASTNode::Var("x".to_string());
        assert_eq!(eval(ast_x.clone(), &environment).unwrap(), Value::Num(5));

        let ast_y = ASTNode::Var("y".to_string());
        assert_eq!(eval(ast_y.clone(), &environment).unwrap(), Value::Num(10));

        // Test arithmetic operations with variables in the environment
        let ast_add = ASTNode::Add(Box::new(ast_x.clone()), Box::new(ast_y.clone()));
        assert_eq!(eval(ast_add, &environment).unwrap(), Value::Num(15));

        let ast_sub = ASTNode::Sub(Box::new(ast_y), Box::new(ast_x));
        assert_eq!(eval(ast_sub, &environment).unwrap(), Value::Num(5));
    }
}

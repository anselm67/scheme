use std::io::{BufReader, Bytes, Read};
use std::iter::Peekable;
use std::path::{Path, PathBuf};

use crate::heap::Handle;
use crate::interp::Interp;
use crate::types::{Number, SchemeError, Value};

pub struct Parser<R: Read> {
    path: Option<PathBuf>,
    lineno: i64,
    reader: Peekable<Bytes<BufReader<R>>>,
}

impl<R: Read> Parser<R> {
    
    pub fn new(reader: R) -> Self {
        Self {
            path: None,
            lineno: 1,
            reader: BufReader::new(reader).bytes().peekable(),
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.reader.peek()?.as_ref().ok().cloned()
    }

    fn next(&mut self) -> Option<u8> {
        let ch = self.reader.next()?.ok();
        if let Some(lf) = ch && lf == b'\n' {
            self.lineno += 1;
        }
        ch
    }

    fn syntax_error<T>(&self, msg: String) -> Result<T, SchemeError> {
        let path_str = if let Some(ref path) = self.path { 
            path.to_string_lossy().to_string() 
        } else { 
            "unknown".to_string()
        };
        Err(SchemeError::SyntaxError(format!("{}:{}: syntax error: {}", 
            path_str, self.lineno, msg)))
    }

    fn check_for(&mut self, expected: u8) -> Result<Value, SchemeError> {
        match self.peek() {
            Some(actual) if actual == expected => {self.next(); Ok(Value::Unbound) },
            Some(actual) => self.syntax_error(format!(
                "Expected '{}', found {}", expected as char, actual as char
            )),
            None => self.syntax_error(format!(
                "Expected '{}', but reached end of file.", expected as char
            ))
        }
    }

    fn is_whitespace(&self, ch: u8) -> bool {
        ch.is_ascii_whitespace()
    }

    fn is_symbol(&self, ch: u8) -> bool {
        matches!(ch, b'a'..=b'z' | b'A'..=b'Z' 
            | b'+' | b'-' | b'*' | b'/'| b'>' | b'<'| b'=' | b'%'
            | b'!' | b'?')
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if self.is_whitespace(ch) {
                self.next();
            } else if ch == b';' {
                // Skip comment until end of line
                while let Some(n) = self.next() {
                    if n == b'\n' { break; }
                }
            } else {
                break;
            }
        }
    }

    fn parse_number_with_sign(&mut self, sign: Option<u8>) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        if let Some(ch) = sign {
            token.push(ch as char);
        }
        let mut has_dot = false;
        let mut has_exponent = false;

        // Swallows the optional sign.
        if let Some(ch) = self.peek() && (ch == b'-' || ch == b'+') {
            token.push(ch as char);
            self.next();
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                token.push(ch as char);
                self.next();
            } else if ch == b'.' && !has_dot && ! has_exponent {
                has_dot = true;
                token.push(ch as char);
                self.next();
            } else if ch == b'e' || ch == b'E' && ! has_exponent {
                has_exponent = true;
                token.push(ch as char);
                self.next();
                // Exponent sign
                if let Some(next_ch) = self.peek() && (next_ch == b'-' || next_ch == b'+') {
                    token.push(next_ch as char);
                    self.next();
                }
            } else {
                break;
            }
        }
        if has_dot || has_exponent {
            match token.parse::<f64>() {
                Ok(num) => Ok(Handle::Value(Value::Number(Number::Float(num)))),
                Err(_) => self.syntax_error(format!("Invalid float number: {}", token)),  
            }
        } else {    
            match token.parse::<i64>() {
                Ok(num) => Ok(Handle::Value(Value::Number(Number::Int(num)))),
                Err(_) => self.syntax_error(format!("Invalid integer number: {}", token)),  
            }
        }
    }

    fn parse_number(&mut self) -> Result<Handle, SchemeError> {
        self.parse_number_with_sign(None)
    }

    fn parse_symbol_with_lead(&mut self, interp: &Interp, lead: Option<u8>) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        if let Some(ch) = lead {
            token.push(ch as char)
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || b"!$%&*/:<=>?^_~+-".contains(&ch) {
                token.push(ch as char);
                self.next();
            } else {
                break;
            }
        }
        return Ok(interp.lookup(&token))
    }

    fn parse_symbol(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        return self.parse_symbol_with_lead(interp, None)
    }

    fn parse_hash_number(&mut self, radix: u32) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        while let Some(byte) = self.peek() {
            let ch = byte as char;
            if ch.is_digit(radix) {
                self.next();
                token.push(ch);
            } else {
                break;
            }
        }
        match i64::from_str_radix(&token, radix) {
            Ok(num) => Ok(Handle::Value(Value::Number(Number::Int(num)))),
            Err(_) => self.syntax_error(format!("Invalid '#xx' number {token}."))
        }
    }

    fn parse_hash_character(&mut self) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        while let Some(ch) = self.peek() {
            let ch = ch as char;
            if ch.is_ascii_alphabetic() {
                self.next();
                token.push(ch);
            } else {
                break;
            }
        }
        if token.len() == 1 {
            Ok(Handle::Value(Value::Char(token.as_bytes()[0])))
        } else {
            match token.to_ascii_lowercase().as_str() {
                "space" => Ok(Handle::Value(Value::Char(32))),
                "backspace" => Ok(Handle::Value(Value::Char(8))),
                "tab" => Ok(Handle::Value(Value::Char(9))),
                "newline" => Ok(Handle::Value(Value::Char(10))),
                "return" => Ok(Handle::Value(Value::Char(13))),
                _ => self.syntax_error(format!("Invalid #\\ token {}.", token))
            }
        }
    }

    fn parse_hash(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        self.check_for(b'#')?;
        match self.next() {
            Some(ch) if ch == b'(' => self.parse_vector(interp),
            Some(ch) if ch.to_ascii_lowercase() == b't' => Ok(Handle::Value(Value::Boolean(true))),
            Some(ch) if ch.to_ascii_lowercase() == b'f' => Ok(Handle::Value(Value::Boolean(false))),
            Some(ch) if ch == b'b' => self.parse_hash_number(2),
            Some(ch) if ch == b'o' => self.parse_hash_number(8),
            Some(ch) if ch == b'd' => self.parse_hash_number(10),
            Some(ch) if ch == b'x' => self.parse_hash_number(16),
            Some(ch) if ch == b'\\' => self.parse_hash_character(),
            Some(ch) => self.syntax_error(format!(
                "Invalid char in # sequence {}", ch as char
            )),
            None => self.syntax_error(format!(
                "Unexpected end of file while parsing a # expression."
            ))
        }
    }

    fn parse_string(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        self.check_for(b'"')?;
        while let Some(ch) = self.peek() {
            self.next();
            if ch == b'"' {
                return Ok(interp.alloc_string(token));
            } else if ch == b'\\' {
                match self.next() {
                    Some(ch) => token.push(ch as char),
                    None => return self.syntax_error(format!(
                        "Unexpected enf of file while parsing string."                    
                    ))
                }
            } else {
                token.push(ch as char);
            }
        }
        self.syntax_error(format!(
            "Unexpected end of file while parsing string."
        ))
    }

    fn parse_list(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        let mut items = Vec::new();
        self.skip_whitespace();
        while let Some(c) = self.peek() {
            match c {
                b')' => {
                    self.check_for(b')')?;
                    return Ok(interp.alloc_list_from_handles(&items));
                },
                b'.' => {
                    self.next();
                    let cdr = self.read(interp)?;
                    self.skip_whitespace();
                    self.check_for(b')')?;
                    let car = interp.alloc_list_from_handles(&items);
                    let tail = interp.last(car.value())?;
                    interp.setcdr(interp.to_object(tail)?, cdr.value())?;
                    return Ok(car);
                },
                _ => {
                    items.push(self.read(interp)?);
                    self.skip_whitespace();
                }
            }
        }
        self.syntax_error(format!(
            "Unexpected end of file while parsing list."
        ))
    }

    fn parse_vector(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        let mut list = Vec::new();
        self.skip_whitespace();
        while let Some(c) = self.peek() {
            if c == b')' { break; }
            list.push(self.read(interp)?);
            self.skip_whitespace();
        }
        self.check_for(b')')?;
        return Ok(interp.alloc_vector_from_handles(&list));
    }

    pub fn read(&mut self, interp: &Interp) -> Result<Handle, SchemeError> {
        self.skip_whitespace();
        let current = self.peek();
        return match current {
            Some(b'(') => {
                self.next(); // consume '('
                self.parse_list(interp)
            },
            Some(ch) if ch == b'+' || ch == b'-' => {
                self.next();
                match self.peek() {
                    Some(next) if next.is_ascii_digit() => {
                        self.parse_number_with_sign(Some(ch) )
                    } ,
                    _ => self.parse_symbol_with_lead(interp, Some(ch))
                }
            },
            Some(ch) if ch.is_ascii_digit() || ch == b'-' || ch == b'+' => {
                self.parse_number()
            },
            Some(ch) if self.is_symbol(ch) => {
                self.parse_symbol(interp)
            },
            Some(ch) if ch == b'#' => {
                self.parse_hash(interp)
            },
            Some(b'"') => {
                return self.parse_string(interp)
            },
            Some(ch) if ch == b'`' => {
                self.next();
                let value = self.read(interp)?;
                interp.quasiquote( value)
            },
            Some(ch) if ch == b',' => {
                self.next();
                let retval = match self.peek() {
                    Some(b'@') => {
                        self.next();
                        interp.unquote_splicing(self.read(interp)?)
                    },
                    _ => interp.unquote(self.read(interp)?)
                };
                self.skip_whitespace();
                retval
            },
            Some(ch) if ch == b'\'' => {
                self.next();
                interp.quote_from_handle(self.read(interp)?)
            },
            Some(ch) => {
                self.next();
                self.syntax_error(format!("Unexpected character {}", ch as char))
            },
            None => Ok(Handle::Value(Value::Eof)),
        };
    }
}

impl Parser<std::fs::File> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemeError> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)
            .map_err(|e| SchemeError::FileNotFound(e.to_string()))?;
        Ok(Self {
            path: Some(path.to_path_buf()),
            lineno: 1,
            reader: BufReader::new(file).bytes().peekable()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let inputs = vec!["42", "-3", "0", "3.14", "-0.001", "2e10", "-1.5E-3"];
        let expected = vec![
            Value::Number(Number::Int(42)),
            Value::Number(Number::Int(-3)),
            Value::Number(Number::Int(0)),              
            Value::Number(Number::Float(3.14)),
            Value::Number(Number::Float(-0.001)),
            Value::Number(Number::Float(2e10)),
            Value::Number(Number::Float(-1.5e-3)),
        ];  
        for (input, expect) in inputs.iter().zip(expected.iter()) {
            let mut parser = Parser::new(input.as_bytes());
            let result = parser.parse_number().unwrap();
            assert_eq!(&result.value(), expect);
        }
    }

    #[test]
    fn test_parse_hash() {
        let ok_inputs = vec![
            ("#t", Value::Boolean(true)),
            ("#f", Value::Boolean(false)),
            ("#T", Value::Boolean(true)),
            ("#F", Value::Boolean(false)),
            ("#d10", Value::Number(Number::Int(10))),
            ("#b101", Value::Number(Number::Int(5))),
            ("#o10", Value::Number(Number::Int(8))),
            ("#xFF", Value::Number(Number::Int(255))),
            ("#\\backspace", Value::Char(8)),
            ("#\\tab", Value::Char(9)),
            ("#\\newline", Value::Char(10)),
            ("#\\return", Value::Char(13)),
            ("#\\space", Value::Char(32)),
            ("#\\A", Value::Char(65)),
        ];
        let interp = Interp::new();
        for (text, value) in ok_inputs {
            let mut parser = Parser::new(text.as_bytes());
            assert_eq!(value, parser.parse_hash(&interp).expect("valid input").value())
        }
    }

    #[test]
    fn test_parse_symbol() {
        let interp = Interp::new();
        let inputs = vec![
            "some-symbol",
        ];
        for text in inputs {
            let mut parser = Parser::new(text.as_bytes());
            let result = parser.parse_symbol(&interp);
            assert!(matches!(result.expect("valid symbol").value(), Value::Object(_id)));
        }
    }

    #[test]
    fn test_parse_string() {
        let interp = Interp::new();
        let inputs = vec![
            "\"Hello World\"",
        ];
        for text in inputs {
            let mut parser = Parser::new(text.as_bytes());
            let result = parser.parse_string(&interp);
            assert!(matches!(result.expect("valid string").value(), Value::Object(_id)));
        }
    }

    #[test]
    fn test_parse_list() {
        let interp = Interp::new();
        let inputs = vec![
            "1 . 2)",
            ")",
            "1 2 3)"
        ];
        for text in inputs {
            let mut parser = Parser::new(text.as_bytes());
            let result = parser.parse_list(&interp);
            assert!(result.is_ok());
        }
    }
}

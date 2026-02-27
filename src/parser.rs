use std::io::{BufReader, Bytes, Cursor, Read};
use std::iter::Peekable;
use std::path::Path;

use crate::heap::Handle;
use crate::interp::Scheme;
use crate::types::{Location, Number, SchemeError, Value};

pub struct Parser<'a> {
    location: Location,
    last_location: Location,
    reader: Peekable<Bytes<Box<dyn Read + 'a>>>,
}

impl<'a> Parser<'a> {
    pub fn new(reader: Box<dyn Read + 'a>) -> Self {
        let location = Location {
            source: "unknown".to_string(),
            lineno: 1,
        };
        Self {
            location: location.clone(),
            last_location: location.clone(),
            reader: reader.bytes().peekable(),
        }
    }

    fn new_with_name(source: &str, reader: Box<dyn Read + 'a>) -> Self {
        let location = Location {
            source: source.to_string(),
            lineno: 1,
        };
        Self {
            location: location.clone(),
            last_location: location.clone(),
            reader: reader.bytes().peekable(),
        }
    }

    pub fn from_reader(reader: Box<dyn Read + 'a>) -> Self {
        Parser::new(reader)
    }

    pub fn from_borrowed(reader: &'a mut dyn Read) -> Self {
        Parser::new(Box::new(reader))
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemeError> {
        let path = path.as_ref();
        let source = path.to_str().unwrap_or("<invalid-path>");
        let file =
            std::fs::File::open(path).map_err(|e| SchemeError::FileNotFound(e.to_string()))?;
        let buffered = BufReader::new(file);
        let reader: Box<dyn Read + 'a> = Box::new(buffered);
        Ok(Parser::new_with_name(source, reader))
    }

    pub fn from_string(content: &'a str) -> Self {
        Parser::new_with_name(
            "<string>",
            Box::new(Cursor::new(content)) as Box<dyn Read + 'a>,
        )
    }

    fn peek(&mut self) -> Option<u8> {
        self.reader.peek()?.as_ref().ok().cloned()
    }

    fn next(&mut self) -> Option<u8> {
        let ch = self.reader.next()?.ok();
        if let Some(lf) = ch
            && lf == b'\n'
        {
            self.location.lineno += 1;
        }
        ch
    }

    fn syntax_error<T>(&self, msg: String) -> Result<T, SchemeError> {
        Err(SchemeError::SyntaxError(format!(
            "{}:{}: syntax error: {}",
            self.location.source, self.location.lineno, msg
        )))
    }

    fn check_for(&mut self, expected: u8) -> Result<Value, SchemeError> {
        match self.peek() {
            Some(actual) if actual == expected => {
                self.next();
                Ok(Value::Unbound)
            }
            Some(actual) => self.syntax_error(format!(
                "Expected '{}', found {}",
                expected as char, actual as char
            )),
            None => self.syntax_error(format!(
                "Expected '{}', but reached end of file.",
                expected as char
            )),
        }
    }

    fn is_whitespace(&self, ch: u8) -> bool {
        ch.is_ascii_whitespace()
    }

    fn is_symbol(&self, ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || b"?+-.!$%&*:/<=>~^_".contains(&ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if self.is_whitespace(ch) {
                self.next();
            } else if ch == b';' {
                // Skip comment until end of line
                while let Some(n) = self.next() {
                    if n == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn parse_number_with_sign(
        &mut self,
        interp: &Scheme,
        sign: Option<u8>,
    ) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        if let Some(ch) = sign {
            token.push(ch as char);
        }
        let mut has_dot = false;
        let mut has_exponent = false;

        // Swallows the optional sign.
        if let Some(ch) = self.peek()
            && (ch == b'-' || ch == b'+')
        {
            token.push(ch as char);
            self.next();
        }
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                token.push(ch as char);
                self.next();
            } else if ch == b'.' && !has_dot && !has_exponent {
                has_dot = true;
                token.push(ch as char);
                self.next();
            } else if ch == b'e' || ch == b'E' && !has_exponent {
                has_exponent = true;
                token.push(ch as char);
                self.next();
                // Exponent sign
                if let Some(next_ch) = self.peek()
                    && (next_ch == b'-' || next_ch == b'+')
                {
                    token.push(next_ch as char);
                    self.next();
                }
            } else {
                break;
            }
        }
        if has_dot || has_exponent {
            match token.parse::<f64>() {
                Ok(num) => Ok(interp.handle(Value::Number(Number::Float(num)))),
                Err(_) => self.syntax_error(format!("Invalid float number: {}", token)),
            }
        } else {
            match token.parse::<i64>() {
                Ok(num) => Ok(interp.handle(Value::Number(Number::Int(num)))),
                Err(_) => self.syntax_error(format!("Invalid integer number: {}", token)),
            }
        }
    }

    fn parse_number(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.parse_number_with_sign(interp, None)
    }

    fn parse_symbol_with_lead(
        &mut self,
        interp: &Scheme,
        lead: Option<u8>,
    ) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        if let Some(ch) = lead {
            token.push(ch as char)
        }
        while let Some(ch) = self.peek() {
            if self.is_symbol(ch) {
                token.push(ch as char);
                self.next();
            } else {
                break;
            }
        }
        return Ok(interp.lookup(&token.to_lowercase()));
    }

    fn parse_symbol(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        return self.parse_symbol_with_lead(interp, None);
    }

    fn parse_hash_number(&mut self, interp: &Scheme, radix: u32) -> Result<Handle, SchemeError> {
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
            Ok(num) => Ok(interp.handle(Value::Number(Number::Int(num)))),
            Err(_) => self.syntax_error(format!("Invalid '#xx' number {token}.")),
        }
    }

    fn parse_hash_character(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        while let Some(ch) = self.peek() {
            let ch = ch as char;
            if ch.is_alphanumeric() {
                self.next();
                token.push(ch);
            } else if matches!(ch, ' ' | ';' | '(' | '.' | ')' | '*' | '?' | '\\' | '"')
                && token.is_empty()
            {
                self.next();
                token.push(ch);
                break;
            } else {
                break;
            }
        }
        if token.len() == 1 {
            Ok(interp.handle(Value::Char(token.as_bytes()[0])))
        } else {
            match token.to_ascii_lowercase().as_str() {
                "space" => Ok(interp.handle(Value::Char(32))),
                "backspace" => Ok(interp.handle(Value::Char(8))),
                "tab" => Ok(interp.handle(Value::Char(9))),
                "newline" => Ok(interp.handle(Value::Char(10))),
                "return" => Ok(interp.handle(Value::Char(13))),
                _ => self.syntax_error(format!("Invalid #\\ token \"{}\".", token)),
            }
        }
    }

    fn parse_hash(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.check_for(b'#')?;
        match self.next() {
            Some(ch) if ch == b'(' => self.parse_vector(interp),
            Some(ch) if ch.to_ascii_lowercase() == b't' => Ok(interp.handle(Value::Boolean(true))),
            Some(ch) if ch.to_ascii_lowercase() == b'f' => Ok(interp.handle(Value::Boolean(false))),
            Some(ch) if ch == b'b' => self.parse_hash_number(interp, 2),
            Some(ch) if ch == b'o' => self.parse_hash_number(interp, 8),
            Some(ch) if ch == b'd' => self.parse_hash_number(interp, 10),
            Some(ch) if ch == b'x' => self.parse_hash_number(interp, 16),
            Some(ch) if ch == b'\\' => self.parse_hash_character(interp),
            Some(ch) => self.syntax_error(format!("Invalid char in # sequence {}", ch as char)),
            None => self.syntax_error(format!(
                "Unexpected end of file while parsing a # expression."
            )),
        }
    }

    fn parse_string(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        self.check_for(b'"')?;
        while let Some(ch) = self.peek() {
            self.next();
            if ch == b'"' {
                return Ok(interp.alloc_string(token));
            } else if ch == b'\\' {
                match self.next() {
                    Some(ch) if ch == b'n' => token.push('\n'),
                    Some(ch) if ch == b'r' => token.push('\r'),
                    Some(ch) if ch == b't' => token.push('\t'),
                    Some(ch) => token.push(ch as char),
                    None => {
                        return self
                            .syntax_error(format!("Unexpected enf of file while parsing string."));
                    }
                }
            } else {
                token.push(ch as char);
            }
        }
        self.syntax_error(format!("Unexpected end of file while parsing string."))
    }

    fn parse_list(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut items = Vec::new();
        self.skip_whitespace();
        while let Some(c) = self.peek() {
            match c {
                b')' => {
                    self.check_for(b')')?;
                    return Ok(interp.alloc_list_from_handles(&items));
                }
                b'.' => {
                    self.next();
                    if let Some(lead) = self.peek()
                        && self.is_symbol(lead)
                    {
                        let symbol = self.parse_symbol_with_lead(interp, Some(b'.'))?;
                        items.push(symbol);
                    } else {
                        let cdr = self.do_read(interp)?;
                        self.skip_whitespace();
                        self.check_for(b')')?;
                        let car = interp.alloc_list_from_handles(&items);
                        let tail = interp.last(car.value())?;
                        interp.setcdr(interp.to_object(tail)?, cdr.value())?;
                        return Ok(car);
                    }
                }
                _ => {
                    items.push(self.do_read(interp)?);
                    self.skip_whitespace();
                }
            }
        }
        self.syntax_error(format!("Unexpected end of file while parsing list."))
    }

    fn parse_vector(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut list = Vec::new();
        self.skip_whitespace();
        while let Some(c) = self.peek() {
            if c == b')' {
                break;
            }
            list.push(self.do_read(interp)?);
            self.skip_whitespace();
        }
        self.check_for(b')')?;
        return Ok(interp.alloc_vector_from_handles(&list));
    }

    fn do_read(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.skip_whitespace();
        let current = self.peek();
        return match current {
            Some(b'(') => {
                self.next(); // consume '('
                self.parse_list(interp)
            }
            Some(ch) if ch == b'+' || ch == b'-' => {
                self.next();
                match self.peek() {
                    Some(next) if next.is_ascii_digit() => {
                        self.parse_number_with_sign(interp, Some(ch))
                    }
                    _ => self.parse_symbol_with_lead(interp, Some(ch)),
                }
            }
            Some(ch) if ch.is_ascii_digit() || ch == b'-' || ch == b'+' => {
                self.parse_number(interp)
            }
            Some(ch) if self.is_symbol(ch) => self.parse_symbol(interp),
            Some(ch) if ch == b'#' => self.parse_hash(interp),
            Some(b'"') => return self.parse_string(interp),
            Some(ch) if ch == b'`' => {
                self.next();
                let value = self.do_read(interp)?;
                interp.quasiquote(value)
            }
            Some(ch) if ch == b',' => {
                self.next();
                let retval = match self.peek() {
                    Some(b'@') => {
                        self.next();
                        interp.unquote_splicing(self.do_read(interp)?)
                    }
                    _ => interp.unquote(self.do_read(interp)?),
                };
                self.skip_whitespace();
                retval
            }
            Some(ch) if ch == b'\'' => {
                self.next();
                interp.quote_from_handle(self.do_read(interp)?)
            }
            Some(ch) => {
                self.next();
                self.syntax_error(format!("Unexpected character {}", ch as char))
            }
            None => Ok(interp.handle(Value::Eof)),
        };
    }

    pub fn read(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.last_location = self.location.clone();
        self.do_read(interp)
    }

    pub fn last_location(&self) -> &Location {
        &self.last_location
    }

    pub fn current_location(&self) -> &Location {
        &self.location
    }
}

#[cfg(test)]
mod tests {
    use crate::interp::SchemeOptions;

    use super::*;

    #[test]
    fn test_parse_number() {
        let interp = Scheme::new(&SchemeOptions::new());
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
            let mut parser = Parser::from_string(input);
            let result = parser.parse_number(&interp).unwrap();
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
        let interp = Scheme::new(&SchemeOptions::new());
        for (text, value) in ok_inputs {
            let mut parser = Parser::from_string(text);
            assert_eq!(
                value,
                parser.parse_hash(&interp).expect("valid input").value()
            )
        }
    }

    #[test]
    fn test_parse_symbol() {
        let interp = Scheme::new(&SchemeOptions::new());
        let inputs = vec!["some-symbol"];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_symbol(&interp);
            assert!(matches!(
                result.expect("valid symbol").value(),
                Value::Object(_id)
            ));
        }
    }

    #[test]
    fn test_parse_string() {
        let interp = Scheme::new(&SchemeOptions::new());
        let inputs = vec!["\"Hello World\""];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_string(&interp);
            assert!(matches!(
                result.expect("valid string").value(),
                Value::Object(_id)
            ));
        }
    }

    #[test]
    fn test_parse_list() {
        let interp = Scheme::new(&SchemeOptions::new());
        let inputs = vec!["1 . 2)", ")", "1 2 3)"];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_list(&interp);
            assert!(result.is_ok());
        }
    }
}

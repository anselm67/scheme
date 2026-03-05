use std::io::Cursor;
use std::path::Path;

use async_recursion::async_recursion;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};

use crate::heap::Handle;
use crate::interp::Scheme;
use crate::types::{Location, Number, SchemeError, Value};

pub struct Parser<'a> {
    location: Location,
    last_location: Location,
    reader: Box<dyn AsyncBufRead + Unpin + 'a>,
}

impl<'a> Parser<'a> {
    pub fn new(reader: Box<dyn AsyncBufRead + Unpin + 'a>) -> Self {
        let location = Location {
            source: "unknown".to_string(),
            lineno: 1,
        };
        Self {
            location: location.clone(),
            last_location: location.clone(),
            reader: reader,
        }
    }

    fn new_with_name(source: &str, reader: Box<dyn AsyncBufRead + Unpin + 'a>) -> Self {
        let location = Location {
            source: source.to_string(),
            lineno: 1,
        };
        Self {
            location: location.clone(),
            last_location: location.clone(),
            reader: reader,
        }
    }

    pub fn from_reader(reader: Box<dyn AsyncBufRead + Unpin + 'a>) -> Self {
        Parser::new(reader)
    }

    pub fn from_borrowed(reader: &'a mut (dyn AsyncBufRead + Unpin)) -> Self {
        Parser::new(Box::new(reader))
    }

    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemeError> {
        let path = path.as_ref();
        let source = path.to_str().unwrap_or("<invalid-path>");
        let file = File::open(path)
            .await
            .map_err(|e| SchemeError::FileNotFound(e.to_string()))?;
        let buffered = BufReader::new(file);
        let reader: Box<dyn AsyncBufRead + Unpin + 'a> = Box::new(buffered);
        Ok(Parser::new_with_name(source, reader))
    }

    pub fn from_string(content: &'a str) -> Self {
        Parser::new_with_name(
            "<string>",
            Box::new(Cursor::new(content)) as Box<dyn AsyncBufRead + Unpin + 'a>,
        )
    }

    async fn peek(&mut self) -> Result<Option<u8>, SchemeError> {
        let buf = self
            .reader
            .fill_buf()
            .await
            .map_err(|e| SchemeError::IOError(format!("IO Error parsing input: {e}")))?;
        Ok(if buf.len() == 0 {
            None
        } else {
            buf.first().copied()
        })
    }

    async fn next(&mut self) -> Result<Option<u8>, SchemeError> {
        let ch = self.peek().await?;
        self.reader.consume(1);
        if let Some(lf) = ch
            && lf == b'\n'
        {
            self.location.lineno += 1;
        }
        Ok(ch)
    }

    fn syntax_error<T>(&self, msg: String) -> Result<T, SchemeError> {
        Err(SchemeError::SyntaxError(format!(
            "{}:{}: syntax error: {}",
            self.location.source, self.location.lineno, msg
        )))
    }

    async fn check_for(&mut self, expected: u8) -> Result<Value, SchemeError> {
        match self.peek().await? {
            Some(actual) if actual == expected => {
                self.next().await?;
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

    async fn skip_whitespace(&mut self) -> Result<(), SchemeError> {
        while let Some(ch) = self.peek().await? {
            if self.is_whitespace(ch) {
                self.next().await?;
            } else if ch == b';' {
                // Skip comment until end of line
                while let Some(n) = self.next().await? {
                    if n == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn parse_number_with_sign(
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
        if let Some(ch) = self.peek().await?
            && (ch == b'-' || ch == b'+')
        {
            token.push(ch as char);
            self.next().await?;
        }
        while let Some(ch) = self.peek().await? {
            if ch.is_ascii_digit() {
                token.push(ch as char);
                self.next().await?;
            } else if ch == b'.' && !has_dot && !has_exponent {
                has_dot = true;
                token.push(ch as char);
                self.next().await?;
            } else if ch == b'e' || ch == b'E' && !has_exponent {
                has_exponent = true;
                token.push(ch as char);
                self.next().await?;
                // Exponent sign
                if let Some(next_ch) = self.peek().await?
                    && (next_ch == b'-' || next_ch == b'+')
                {
                    token.push(next_ch as char);
                    self.next().await?;
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

    async fn parse_number(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.parse_number_with_sign(interp, None).await
    }

    async fn parse_symbol_with_lead(
        &mut self,
        interp: &Scheme,
        lead: Option<u8>,
    ) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        if let Some(ch) = lead {
            token.push(ch as char)
        }
        while let Some(ch) = self.peek().await? {
            if self.is_symbol(ch) {
                token.push(ch as char);
                self.next().await?;
            } else {
                break;
            }
        }
        return Ok(interp.lookup(&token.to_lowercase()));
    }

    async fn parse_symbol(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        return self.parse_symbol_with_lead(interp, None).await;
    }

    async fn parse_hash_number(
        &mut self,
        interp: &Scheme,
        radix: u32,
    ) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        while let Some(byte) = self.peek().await? {
            let ch = byte as char;
            if ch.is_digit(radix) {
                self.next().await?;
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

    async fn parse_hash_character(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        while let Some(ch) = self.peek().await? {
            let ch = ch as char;
            if ch.is_alphanumeric() {
                self.next().await?;
                token.push(ch);
            } else if matches!(ch, ' ' | ';' | '(' | '.' | ')' | '*' | '?' | '\\' | '"')
                && token.is_empty()
            {
                self.next().await?;
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

    #[async_recursion(?Send)]
    async fn parse_hash(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.check_for(b'#').await?;
        match self.next().await? {
            Some(ch) if ch == b'(' => self.parse_vector(interp).await,
            Some(ch) if ch.to_ascii_lowercase() == b't' => Ok(interp.handle(Value::Boolean(true))),
            Some(ch) if ch.to_ascii_lowercase() == b'f' => Ok(interp.handle(Value::Boolean(false))),
            Some(ch) if ch == b'b' => self.parse_hash_number(interp, 2).await,
            Some(ch) if ch == b'o' => self.parse_hash_number(interp, 8).await,
            Some(ch) if ch == b'd' => self.parse_hash_number(interp, 10).await,
            Some(ch) if ch == b'x' => self.parse_hash_number(interp, 16).await,
            Some(ch) if ch == b'\\' => self.parse_hash_character(interp).await,
            Some(ch) => self.syntax_error(format!("Invalid char in # sequence {}", ch as char)),
            None => self.syntax_error(format!(
                "Unexpected end of file while parsing a # expression."
            )),
        }
    }

    async fn parse_string(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut token = String::new();
        self.check_for(b'"').await?;
        while let Some(ch) = self.peek().await? {
            self.next().await?;
            if ch == b'"' {
                return Ok(interp.alloc_string(token));
            } else if ch == b'\\' {
                match self.next().await? {
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

    #[async_recursion(?Send)]
    async fn parse_list(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut items = Vec::new();
        self.skip_whitespace().await?;
        while let Some(c) = self.peek().await? {
            match c {
                b')' => {
                    self.check_for(b')').await?;
                    return Ok(interp.alloc_list_from_handles(&items));
                }
                b'.' => {
                    self.next().await?;
                    if let Some(lead) = self.peek().await?
                        && self.is_symbol(lead)
                    {
                        let symbol = self.parse_symbol_with_lead(interp, Some(b'.')).await?;
                        items.push(symbol);
                    } else {
                        let cdr = self.do_read(interp).await?;
                        self.skip_whitespace().await?;
                        self.check_for(b')').await?;
                        let car = interp.alloc_list_from_handles(&items);
                        let tail = interp.last(car.value())?;
                        interp.setcdr(interp.to_object(tail)?, cdr.value())?;
                        return Ok(car);
                    }
                }
                _ => {
                    items.push(self.do_read(interp).await?);
                    self.skip_whitespace().await?;
                }
            }
        }
        self.syntax_error(format!("Unexpected end of file while parsing list."))
    }

    async fn parse_vector(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        let mut list = Vec::new();
        self.skip_whitespace().await?;
        while let Some(c) = self.peek().await? {
            if c == b')' {
                break;
            }
            list.push(self.do_read(interp).await?);
            self.skip_whitespace().await?;
        }
        self.check_for(b')').await?;
        return Ok(interp.alloc_vector_from_handles(&list));
    }

    #[async_recursion(?Send)]
    async fn do_read(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.skip_whitespace().await?;
        let current = self.peek().await?;
        return match current {
            Some(b'(') => {
                self.next().await?; // consume '('
                self.parse_list(interp).await
            }
            Some(ch) if ch == b'+' || ch == b'-' => {
                self.next().await?;
                match self.peek().await? {
                    Some(next) if next.is_ascii_digit() => {
                        self.parse_number_with_sign(interp, Some(ch)).await
                    }
                    _ => self.parse_symbol_with_lead(interp, Some(ch)).await,
                }
            }
            Some(ch) if ch.is_ascii_digit() || ch == b'-' || ch == b'+' => {
                self.parse_number(interp).await
            }
            Some(ch) if self.is_symbol(ch) => self.parse_symbol(interp).await,
            Some(ch) if ch == b'#' => self.parse_hash(interp).await,
            Some(b'"') => return self.parse_string(interp).await,
            Some(ch) if ch == b'`' => {
                self.next().await?;
                let value = self.do_read(interp).await?;
                interp.quasiquote(value)
            }
            Some(ch) if ch == b',' => {
                self.next().await?;
                let retval = match self.peek().await? {
                    Some(b'@') => {
                        self.next().await?;
                        interp.unquote_splicing(self.do_read(interp).await?)
                    }
                    _ => interp.unquote(self.do_read(interp).await?),
                };
                self.skip_whitespace().await?;
                retval
            }
            Some(ch) if ch == b'\'' => {
                self.next().await?;
                interp.quote_from_handle(self.do_read(interp).await?)
            }
            Some(ch) => {
                self.next().await?;
                self.syntax_error(format!("Unexpected character {}", ch as char))
            }
            None => Ok(interp.handle(Value::Eof)),
        };
    }

    pub async fn read(&mut self, interp: &Scheme) -> Result<Handle, SchemeError> {
        self.last_location = self.location.clone();
        self.do_read(interp).await
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

    #[tokio::test]
    async fn test_parse_number() {
        let interp = Scheme::new(&SchemeOptions::new()).await;
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
            let result = parser.parse_number(&interp).await.unwrap();
            assert_eq!(&result.value(), expect);
        }
    }

    #[tokio::test]
    async fn test_parse_hash() {
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
        let interp = Scheme::new(&SchemeOptions::new()).await;
        for (text, value) in ok_inputs {
            let mut parser = Parser::from_string(text);
            assert_eq!(
                value,
                parser
                    .parse_hash(&interp)
                    .await
                    .expect("valid input")
                    .value()
            )
        }
    }

    #[tokio::test]
    async fn test_parse_symbol() {
        let interp = Scheme::new(&SchemeOptions::new()).await;
        let inputs = vec!["some-symbol"];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_symbol(&interp);
            assert!(matches!(
                result.await.expect("valid symbol").value(),
                Value::Object(_id)
            ));
        }
    }

    #[tokio::test]
    async fn test_parse_string() {
        let interp = Scheme::new(&SchemeOptions::new()).await;
        let inputs = vec!["\"Hello World\""];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_string(&interp);
            assert!(matches!(
                result.await.expect("valid string").value(),
                Value::Object(_id)
            ));
        }
    }

    #[tokio::test]
    async fn test_parse_list() {
        let interp = Scheme::new(&SchemeOptions::new()).await;
        let inputs = vec!["1 . 2)", ")", "1 2 3)"];
        for text in inputs {
            let mut parser = Parser::from_string(text);
            let result = parser.parse_list(&interp);
            assert!(result.await.is_ok());
        }
    }
}

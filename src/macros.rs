
#[macro_export]
macro_rules! extract_args {
    ($args:expr, $count:expr, $($name:ident : $variant:ident),*) => {
        if $args.len() != $count {
            return Err(SchemeError::ArgCountError(format!(
                "Invalid arg-count {} expected {}.", 
                $args.len(), $count))
            );
        }
        let mut iter = $args.into_iter();
        $(
            let $name = match iter.next().unwrap() {
                Value::$variant(val) => val,
                _ => return Err(SchemeError::TypeError(format!(
                    "Invalid type {}.", stringify!($variant).to_string()))),
            };
        )*
    };
}

#[macro_export]
macro_rules! all_numbers {
    ($args:expr) => {
        $args.into_iter().map(|v| match v {
            Value::Number(n) => Ok(n),
            _ => Err(SchemeError::TypeError(format!("Expected a Number."))),
        }).collect::<Result<Vec<_>, _>>()?
    };
}
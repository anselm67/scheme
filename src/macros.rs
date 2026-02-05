
#[macro_export]
macro_rules! check_arity {
    ($args:expr, $count: expr) => {
        if $args.len() != $count {
            return Err(SchemeError::ArgCountError(format!(
                "Expected {} args, but got {}.", $count, $args.len()
            )))
        }
    }
}

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
macro_rules! all_of_type {
    ($args:expr, $variant:path, $type_name:expr) => {
        $args.into_iter().map(|v| match v {
            $variant(inner) => Ok(*inner),
            _ => Err(SchemeError::TypeError($type_name.to_string())),
        }).collect::<Result<Vec<_>, SchemeError>>()?
    };
}
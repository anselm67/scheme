
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
macro_rules! check_min_arity {
    ($args:expr, $count: expr) => {
        if $args.len() < $count {
            return Err(SchemeError::ArgCountError(format!(
                "Expected at least {} args, but got {}.", $count, $args.len()
            )))
        }
    }
}

#[macro_export]
macro_rules! check_arity_range {
    ($args:expr, $low: expr, $high: expr) => {
        let actual = $args.len() as isize;
        if actual < $low || $args.len() > $high {
            return Err(SchemeError::ArgCountError(format!(
                "Expected {} to {} args, but got {}.", $low, $high, $args.len()
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
                // If you call it with : Value, it matches this branch
                // val if stringify!($variant) == "Any" => val,
                // Otherwise it matches the specific variant
                Value::$variant(v) => v,
                other => return Err(SchemeError::TypeError(format!(
                    "Expected {}, got {:?}", stringify!($variant), other
                ))),
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
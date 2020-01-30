#[macro_export]
macro_rules! evaluate {
    ($e: expr, $i: ident, $err: expr, $e_ident: ident) => {
        let mut results: Vec<BynarResult<()>> = Vec::new();
        for unit in &$e.$i {
            if unit.health() != "OK" {
                // unit failed
                let err = format!($expr, unit.$e_ident);
                results.push(Err(BynarError::new(err)));
            }
        }
        results
    };
}

// parse object of type type_name from vec<u8> mess
#[macro_export]
macro_rules! get_message {
    ($type_name:ty, $mess:expr) => {
        parse_from_bytes::<api::service::$type_name>($mess)
    };
}

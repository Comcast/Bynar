#[macro_export]
macro_rules! eval {
    ($results: expr, $e: expr, $err: expr, $e_ident: ident) => {
        for unit in $e {
            if unit.health() != "OK" {
                // unit failed
                let err = format!($err, unit.$e_ident);
                $results.push(Err(BynarError::new(err)));
            }
        }
    };
    ($results: expr, $e: expr, $err: expr, $e_ident: ident, $e_ident2: ident) => {
        for unit in $e {
            if unit.health() != "OK" {
                // unit failed
                let err = format!($err, unit.$e_ident, unit.$e_ident2);
                $results.push(Err(BynarError::new(err)));
            }
        }
    };
    ($results: expr, $e: expr, $comp: expr, $err: expr, $e_ident: ident) => {
        for unit in $e {
            if unit.health() != "OK" && unit.health() != $comp {
                // unit failed
                let err = format!($err, unit.$e_ident);
                $results.push(Err(BynarError::new(err)));
            }
        }
    };
}

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

#[macro_export]
macro_rules! get_results {
    ($v: expr, $i: ident) => {
        $v.into_iter().map($i).collect()
    };
}

#[macro_export]
macro_rules! mult_results {
    ($r: expr, $v: ident, $i: ident) => {
        $i(&$r.$v()?)
    };
}

#[macro_export]
macro_rules! nout_match {
    ($e: expr, $info: expr, $err: expr) => {
        match $e {
                    Ok(_) => {
                        info!($info);
                    }
                    Err(e) => {
                        error!($err, e);
                    }
       }
    }
}
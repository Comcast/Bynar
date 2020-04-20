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
        protobuf::parse_from_bytes::<$type_name>($mess)
    };
}

#[macro_export]
macro_rules! poll_events {
    ($s:expr, $ret:expr) => {
        match $s.get_events() {
            Err(zmq::Error::EBUSY) => {
                debug!("Socket Busy, skip");
                std::thread::sleep(std::time::Duration::from_millis(100));
                $ret;
            }
            Err(e) => {
                error!("Get Client Socket Events errored...{:?}", e);
                return Err(BynarError::from(e));
            }
            Ok(e) => e,
        }
    };
}

#[macro_export]
macro_rules! get_op_result {
    ($op_result:expr, $type_op:ident) => {
        match $op_result.get_result() {
            ResultType::OK => return Ok($op_result.get_outcome()),
            ResultType::ERR => {
                if $op_result.has_error_msg() {
                    return Err(BynarError::from($op_result.get_error_msg()));
                } else {
                    return Err(BynarError::from("$type_op failed but error_msg not set"));
                }
            }
        }
    };
}

#[macro_export]
macro_rules! unpack {
    ($e:expr) => {
        match $e {
            Some(val) => {
                match val {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err.into()))
                }
            },
            None => return None
        }
    };
}
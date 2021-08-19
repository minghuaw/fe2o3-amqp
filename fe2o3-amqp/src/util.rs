// #[macro_export]
// macro_rules! unpack {
//     ($e:expr) => {
//         match $e {
//             Some(val) => match val {
//                 Ok(val) => val,
//                 Err(err) => return Some(Err(err.into())),
//             },
//             None => return None,
//         }
//     };
// }

// #[macro_export]
// macro_rules! unpack_or_eof {
//     ($e:expr) => {
//         match $e {
//             Some(val) => match val {
//                 Ok(val) => val,
//                 Err(err) => return Err(err.into()),
//             },
//             None => return Err(Error::Eof),
//         }
//     };
// }

pub(crate) enum NewType {
    None,
    Symbol,
    Array,
}

impl Default for NewType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub enum IsArrayElement {
    False,
    FirstElement,
    OtherElement,
}

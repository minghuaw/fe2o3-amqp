mod attach;
mod begin;
mod close;
mod detach;
mod disposition;
mod end;
mod flow;
mod open;
mod transfer;

pub use attach::*;
pub use begin::*;
pub use close::*;
pub use detach::*;
pub use disposition::*;
pub use end::*;
pub use flow::*;
pub use open::*;
pub use transfer::*;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Performative {
    Open(Open),
    End(End)
}

#[cfg(test)]
mod tests {
    use super::Performative;

    use super::End;
    use fe2o3_amqp::ser::to_vec;
    use fe2o3_amqp::de::from_reader;

    #[test]
    fn test_untagged_serde() {
        let end = Performative::End(End {error: None});
        let buf = to_vec(&end).unwrap();
        let reader = std::io::Cursor::new(buf);
        let end2: Performative = from_reader(reader).unwrap();
        // println!("{:x?}", buf );
    }
}
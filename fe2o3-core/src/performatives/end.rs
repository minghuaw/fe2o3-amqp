use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::Error;

// <type name="end" class="composite" source="list" provides="frame">
// <descriptor name="amqp:end:list" code="0x00000000:0x00000017"/>
// <field name="error" type="error"/>
// </type>

#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:end:list",
    code = 0x0000_0000_0000_0017,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct End {
    pub error: Option<Error>,
}
#[cfg(test)]
mod tests {
    use fe2o3_amqp::{de::from_slice, ser::to_vec};

    use super::*;

    #[test]
    fn test_serde_end() {
        let end = End { error: None };
        let buf = to_vec(&end).unwrap();
        println!("{:x?}", &buf);
        let end2: End = from_slice(&buf).unwrap();
        println!("{:?}", end2);
    }
}

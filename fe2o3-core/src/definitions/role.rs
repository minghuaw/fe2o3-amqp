use serde::{de, ser};

/// 2.8.1 Role
///
/// <type name="role" class="restricted" source="boolean">
/// </type>
#[derive(Debug, Clone)]
pub enum Role {
    /// <choice name="sender" value="false"/>
    Sender,
    /// <choice name="receiver" value="true"/>
    Receiver,
}

impl From<Role> for bool {
    fn from(role: Role) -> Self {
        match role {
            Role::Sender => false,
            Role::Receiver => true,
        }
    }
}

impl From<&Role> for bool {
    fn from(role: &Role) -> Self {
        match role {
            Role::Sender => false,
            Role::Receiver => true,
        }
    }
}

impl From<bool> for Role {
    fn from(b: bool) -> Self {
        match b {
            false => Role::Sender,
            true => Role::Receiver,
        }
    }
}

impl ser::Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        bool::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Role;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum Role")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Role::from(v))
    }
}

impl<'de> de::Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_bool(Visitor {})
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp::{from_slice, ser::to_vec};

    use super::Role;

    #[test]
    fn test_role() {
        let role = Role::Sender;
        let buf = to_vec(&role).unwrap();
        println!("{:x?}", buf);
        let role2: Role = from_slice(&buf).unwrap();
        println!("{:?}", role2);
    }
}

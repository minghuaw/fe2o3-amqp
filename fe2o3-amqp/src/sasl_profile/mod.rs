use url::Url;

pub mod error;

pub use error::Error;

use crate::frames::sasl;

#[derive(Debug, Clone)]
pub enum SaslProfile {
    Anonymous,
    Plain {
        username: String,
        password: String,
    }
}

impl<'a> TryFrom<&'a Url> for SaslProfile {
    type Error = &'a Url;

    fn try_from(value: &'a Url) -> Result<Self, Self::Error> {
        match (value.username(), value.password()){
            ("", _) | (_, None) => Err(value),
            (username, Some(password)) => {
                Ok(SaslProfile::Plain {
                    username: username.to_string(),
                    password: password.to_string()
                })
            }
        }
    }
}

impl SaslProfile {
    pub async fn on_frame<W>(writer: &mut W, frame: sasl::Frame) -> Result<(), Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::SaslProfile;

    #[test]
    fn test_try_from_address() {
        let url = "amqps://username:password@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_ok());

        let url = "amqps://:password@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());

        let url = "amqps://username:@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());

        let url = "amqps://@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());
    }
}
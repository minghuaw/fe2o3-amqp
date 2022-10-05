//! Experimental implementation of AMQP 1.0 CBS extension protocol

pub mod client;
pub mod constants;
pub mod put_token;

// /// Open connection with `SaslProfile::Anonymous` and then perform CBS
// pub async fn open_connection_and_perform_cbs<'a, Mode, Tls>(
//     builder: connection::Builder<'a, Mode, Tls>,
// ) -> Result<ConnectionHandle<()>, ()> {
//     todo!()
// }

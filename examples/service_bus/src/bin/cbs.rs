use std::{
    env,
    time::{Duration, SystemTime},
};

use dotenv::dotenv;
use fe2o3_amqp::{
    connection::ConnectionHandle,
    sasl_profile::SaslProfile,
    types::{
        messaging::{ApplicationProperties, Message, Properties},
        primitives::Value,
    },
    Connection, Receiver, Sender, Session,
};
use hmac::{
    digest::{InvalidLength, KeyInit},
    Hmac, Mac,
};
use sha2::Sha256;

fn get_sas_token(key_name: &str, key_value: &str, request_url: &str, ttl: Duration) -> String {
    let now = SystemTime::now();
    let expiry = now.duration_since(SystemTime::UNIX_EPOCH).unwrap() + ttl;
    let expiry = expiry.as_secs().to_string();

    let encoded_url = urlencoding::encode(request_url);

    let input = format!("{encoded_url}\n{expiry}");
    let sig: Vec<u8> = mac::<Hmac<Sha256>>(key_value.as_bytes(), input.as_bytes())
        .unwrap()
        .as_ref()
        .into();
    let sig = base64::encode(&sig);

    let encoded_sig = urlencoding::encode(&sig);
    let encoded_expiry = urlencoding::encode(&expiry);
    let encoded_key_name = urlencoding::encode(key_name);

    format!(
        "SharedAccessSignature sig={}&se={}&skn={}&sr={}",
        encoded_sig, encoded_expiry, encoded_key_name, encoded_url
    )
}

async fn put_token(
    connection: &mut ConnectionHandle<()>,
    sas_token: String,
    namespace: &str,
    entity: &str,
) -> i32 {
    let mut session = Session::begin(connection).await.unwrap();

    let cbs_client_addr = "cbs-client-reply-to";
    let mut cbs_sender = Sender::attach(&mut session, "cbs-sender", "$cbs")
        .await
        .unwrap();
    let mut cbs_receiver = Receiver::builder()
        .name("cbs-receiver")
        .source("$cbs")
        .target(cbs_client_addr)
        .attach(&mut session)
        .await
        .unwrap();

    let message = Message::builder()
        .properties(
            Properties::builder()
                .message_id("1".to_string())
                .reply_to(cbs_client_addr)
                .build(),
        )
        .application_properties(
            ApplicationProperties::builder()
                .insert("operation", "put-token")
                .insert("type", "servicebus.windows.net:sastoken")
                .insert("name", format!("amqp://{}/{}", namespace, entity))
                .build(),
        )
        .value(sas_token)
        .build();

    cbs_sender
        .send(message)
        .await
        .unwrap()
        .accepted_or("Not accepted")
        .unwrap();

    let delivery = cbs_receiver.recv::<Value>().await.unwrap();
    cbs_receiver.accept(&delivery).await.unwrap();

    let mut message = delivery.into_message();
    let status_code: i32 = message
        .application_properties
        .as_mut()
        .and_then(|m| m.remove("status-code"))
        .map(TryInto::try_into)
        .unwrap()
        .unwrap();

    cbs_sender.close().await.unwrap();
    cbs_receiver.close().await.unwrap();
    session.close().await.unwrap();

    status_code
}

fn mac<M: Mac + KeyInit>(key: &[u8], input: &[u8]) -> Result<impl AsRef<[u8]>, InvalidLength> {
    let mut mac = <M as Mac>::new_from_slice(key)?;
    mac.update(input);
    Ok(mac.finalize().into_bytes())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let queue_name = "q1";

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("test-connection")
        .alt_tls_establishment(true)
        .sasl_profile(SaslProfile::Anonymous)
        .open(&url[..])
        .await
        .unwrap();

    let request_url = format!("http://{}/{}", hostname, queue_name);
    let sas_token = get_sas_token(
        &sa_key_name,
        &sa_key_value,
        &request_url,
        Duration::from_secs(30 * 60),
    );
    let status_code = put_token(&mut connection, sas_token, &hostname, &queue_name).await;
    assert_eq!(status_code, 202);

    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "test-sender", "q1")
        .await
        .unwrap();
    let mut receiver = Receiver::attach(&mut session, "test-receiver", "q1")
        .await
        .unwrap();

    sender
        .send("hello cbs")
        .await
        .unwrap()
        .accepted_or("Not accepted")
        .unwrap();

    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    println!("{:?}", delivery.body());

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}

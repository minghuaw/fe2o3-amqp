use dotenv::dotenv;
use fe2o3_amqp_management::{client::MgmtClient, operations::ReadRequest};
use std::env;

use fe2o3_amqp::{Connection, sasl_profile::SaslProfile, Session, Sender};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let queue_name = env::var("QUEUE_NAME").unwrap();

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true) // ServiceBus uses alternative TLS establishement
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    // let mut mgmt_client = MgmtClient::attach(&mut session, "rust-mgmt-client-1").await.unwrap();
    let mut mgmt_client = MgmtClient::builder()
        .client_node_addr("rust-mgmt-client-1")
        // .management_node_address(format!("{}/{}", queue_name, "$management"))
        .management_node_address("$management")
        .attach(&mut session)
        .await.unwrap();

    let read_request = ReadRequest::name("q1", "com.microsoft:servicebus", None);
    let response = mgmt_client.call(read_request).await.unwrap();
    println!("{:?}", response);

    let sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
        .await
        .unwrap();

    sender.close().await.unwrap();
    mgmt_client.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
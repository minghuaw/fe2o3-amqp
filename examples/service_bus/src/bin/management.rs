use dotenv::dotenv;
use fe2o3_amqp::{Connection, sasl_profile::SaslProfile, Session};
use fe2o3_amqp_management::{client::MgmtClient, operations::entity::{ReadRequest, ReadResponse}};
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true) // ServiceBus uses alternative TLS establishment
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut mgmt_client = MgmtClient::attach(&mut session, "mgmt-client").await.unwrap();

    // Read request
    let read = ReadRequest::name("q1");
    let outcome = mgmt_client.send_request(read, "com.microsoft:queue", None).await.unwrap();
    println!("{:?}", outcome);
    let response = mgmt_client.recv_response::<ReadResponse, _>().await.unwrap();
    println!("{:?}", response);

    mgmt_client.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}
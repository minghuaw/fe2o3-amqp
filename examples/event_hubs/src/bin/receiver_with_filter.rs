use std::env;

use dotenv::dotenv;
use event_hubs::get_event_hub_partitions;
use fe2o3_amqp::types::messaging::Body;
use fe2o3_amqp::{
    sasl_profile::SaslProfile,
    types::{messaging::Source, primitives::Value},
    Connection, Receiver, Session,
};
use fe2o3_amqp_ext::filters::SelectorFilter;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let event_hub_name = env::var("EVENT_HUB_NAME").unwrap();

    let url = format!("amqps://{}", hostname);
    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true) // EventHubs uses alternative TLS establishment
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();

    let partitions = get_event_hub_partitions(&mut connection, &event_hub_name)
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    // <event_hub_name>/ConsumerGroups/$default/Partitions/<partition>;
    let partition = &partitions[0]; // This should be equal to "0"
    let name = format!("receiver-{}", partition);
    let partition_address = format!(
        "{}/ConsumerGroups/$default/Partitions/{}",
        event_hub_name, partition
    );
    let mut receiver = Receiver::builder()
        .name(name)
        .source(
            Source::builder()
                .address(partition_address)
                .add_to_filter(
                    "filter_latest",
                    SelectorFilter::new("amqp.annotation.x-opt-offset > @latest"),
                )
                .build(),
        )
        .attach(&mut session)
        .await
        .unwrap();

    for _ in 0..3 {
        let delivery = receiver.recv::<Body<Value>>().await.unwrap();
        let msg = std::str::from_utf8(&delivery.body().try_as_data().unwrap().next().unwrap()[..])
            .unwrap();
        println!("{:?}", msg);
        receiver.accept(&delivery).await.unwrap();
    }

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}

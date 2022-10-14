//! Tests sending and receiving small messages with active mq artemis

use std::time::Duration;

use fe2o3_amqp::{Connection, Receiver, Sender, Session};
use fe2o3_amqp_types::messaging::Message;
use testcontainers::{clients, images};

#[tokio::test]
async fn activemq_artemis_send_receive() {
    let docker = clients::Cli::default();
    let image = images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
        .with_env_var("DISABLE_SECURITY", "true")
        .with_exposed_port(5672);
    let node = docker.run(image);
    tokio::time::sleep(Duration::from_millis(3_000)).await; // wait for container to start

    let port = node.get_host_port_ipv4(5672);
    let url = format!("amqp://localhost:{}", port);
    let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
        .await
        .unwrap();
    let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
        .await
        .unwrap();

    let message = Message::from("test-message");
    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or("Not accepted").unwrap();

    let received = receiver.recv::<String>().await.unwrap();
    receiver.accept(&received).await.unwrap();
    assert_eq!(received.body(), "test-message");

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test]
async fn activemq_artemis_send_receive_large_content() {
    let docker = clients::Cli::default();
    let image = images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
        .with_env_var("DISABLE_SECURITY", "true")
        .with_exposed_port(5672);
    let node = docker.run(image);
    tokio::time::sleep(Duration::from_millis(3_000)).await; // wait for container to start

    let port = node.get_host_port_ipv4(5672);
    let url = format!("amqp://localhost:{}", port);
    let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
        .await
        .unwrap();
    let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
        .await
        .unwrap();

    let message = Message::from("test-message".repeat(100_000));
    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or("Not accepted").unwrap();

    let received = receiver.recv::<String>().await.unwrap();
    receiver.accept(&received).await.unwrap();
    assert_eq!(received.body(), &"test-message".repeat(100_000));

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}

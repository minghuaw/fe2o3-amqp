use std::time::Duration;

use fe2o3_amqp::Connection;
use testcontainers::{clients, images};

#[tokio::test]
async fn test_single_amqp_connection() {
    let docker = clients::Cli::default();
    let image = images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
        .with_env_var("DISABLE_SECURITY", "true")
        .with_exposed_port(5672);
    let node = docker.run(image);
    tokio::time::sleep(Duration::from_millis(3_000)).await; // wait for container to start

    let port = node.get_host_port_ipv4(5672);
    let url = format!("amqp://localhost:{}", port);
    let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test]
async fn test_sasl_connection() {
    let docker = clients::Cli::default();
    let image = images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
        .with_env_var("ARTEMIS_USERNAME", "test")
        .with_env_var("ARTEMIS_PASSWORD", "test")
        .with_exposed_port(5672);
    let node = docker.run(image);
    tokio::time::sleep(Duration::from_millis(3_000)).await; // wait for container to start

    let port = node.get_host_port_ipv4(5672);
    let url = format!("amqp://test:test@localhost:{}", port);
    let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
    connection.close().await.unwrap();
}

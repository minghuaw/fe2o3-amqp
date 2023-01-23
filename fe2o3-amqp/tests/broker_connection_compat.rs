//! Tests connection with different brokers

macro_rules! cfg_not_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg(not(target_arch = "wasm32"))]
            $item
        )*
    }
}

cfg_not_wasm32! {
    use fe2o3_amqp::connection::Connection;

    mod common;

    #[tokio::test]
    async fn activemq_artemis_connection() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn activemq_artemis_sasl_plain_connection() {
        let (_node, port) = common::setup_activemq_artemis(Some("guest"), Some("guest")).await;
        let url = format!("amqp://guest:guest@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn rabbitmq_amqp10_connection() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn rabbitmq_amqp10_sasl_plain_connection() {
        let (_node, port) = common::setup_rabbitmq_amqp10(Some("guest"), Some("guest")).await;
        let url = format!("amqp://guest1:guest@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }
}

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
    async fn test_connection_compat() {
        activemq_artemis_connection().await;
        activemq_artemis_sasl_plain_connection().await;
        qpid_broker_j_connection().await;
        qpid_broker_j_sasl_plain_connection().await;
        rabbitmq_amqp10_connection().await;
        rabbitmq_amqp10_sasl_plain_connection().await;
    }

    async fn activemq_artemis_connection() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    async fn activemq_artemis_sasl_plain_connection() {
        let (_node, port) = common::setup_activemq_artemis(Some("artemis"), Some("artemis")).await;
        let url = format!("amqp://artemis:artemis@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    async fn qpid_broker_j_connection() {
        // The default config of the image only supports SASL PLAIN with the
        // `admin`/`admin` user. Anonymous access is not enabled.
        let (_node, port) = common::setup_qpid_broker_j(None, None).await;
        let url = format!("amqp://admin:admin@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    async fn qpid_broker_j_sasl_plain_connection() {
        let (_node, port) = common::setup_qpid_broker_j(Some("admin"), Some("admin")).await;
        let url = format!("amqp://admin:admin@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    async fn rabbitmq_amqp10_connection() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }

    async fn rabbitmq_amqp10_sasl_plain_connection() {
        let (_node, port) = common::setup_rabbitmq_amqp10(Some("guest"), Some("guest")).await;
        let url = format!("amqp://guest:guest@localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        connection.close().await.unwrap();
    }
}

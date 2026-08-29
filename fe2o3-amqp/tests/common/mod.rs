use std::path::Path;

use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

/// Runs `fut` under a 60s timeout, panicking with the expression text on
/// timeout or with the error on failure. Bounds the AMQP setup operations
/// (connection open, session begin, link attach) in integration tests: a
/// broker that accepts TCP but stalls the handshake would otherwise hang the
/// test until the CI job timeout.
///
/// Expands to a future so that it can also be joined concurrently with the
/// acceptor-side operation (e.g. `tokio::join!`).
macro_rules! expect_ok {
    ($fut:expr) => {
        async {
            let what = stringify!($fut);
            tokio::time::timeout(std::time::Duration::from_secs(60), $fut)
                .await
                .unwrap_or_else(|_| panic!("AMQP operation timed out after 60s: {what}"))
                .unwrap()
        }
    };
}

pub(crate) use expect_ok;

#[allow(dead_code)] // not used by all test binaries
pub async fn setup_activemq_artemis(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/apache/artemis", "2.55.0-alpine")
                .with_exposed_port(5672.tcp())
                .with_wait_for(WaitFor::message_on_stdout("AMQ221007: Server is now"))
                .with_env_var("ARTEMIS_USER", username)
                .with_env_var("ARTEMIS_PASSWORD", password)
        }
        _ => GenericImage::new("docker.io/apache/artemis", "2.55.0-alpine")
            .with_exposed_port(5672.tcp())
            .with_wait_for(WaitFor::message_on_stdout("AMQ221007: Server is now"))
            .with_env_var("ANONYMOUS_LOGIN", "true"),
    };
    let node = image.start().await.unwrap();

    let port = node.get_host_port_ipv4(5672).await.unwrap();
    (node, port)
}

#[allow(dead_code)] // not used by all test binaries
pub async fn setup_qpid_broker_j(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    // The default config of the image only supports SASL PLAIN with a single
    // user configured via `QPID_ADMIN_USER`/`QPID_ADMIN_PASSWORD` (defaulting
    // to `admin`/`admin`). Anonymous access is not enabled.
    //
    // The broker does not auto-create queues by default. A custom virtualhost
    // config with a `nodeAutoCreationPolicy` is provided to auto-create queues
    // on publish and consume. The config is copied into the `work-override`
    // folder, whose contents replace the default files on first startup.
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("qpid")
        .join("default.json");
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/apache/qpid-broker-j", "10.1.0")
                .with_exposed_port(5672.tcp())
                .with_wait_for(WaitFor::message_on_stdout("BRK-1004 : Qpid Broker Ready"))
                .with_env_var("QPID_ADMIN_USER", username)
                .with_env_var("QPID_ADMIN_PASSWORD", password)
                .with_copy_to(
                    "/qpid-broker-j/work-override/default.json",
                    config_path.as_path(),
                )
        }
        _ => GenericImage::new("docker.io/apache/qpid-broker-j", "10.1.0")
            .with_exposed_port(5672.tcp())
            .with_wait_for(WaitFor::message_on_stdout("BRK-1004 : Qpid Broker Ready"))
            .with_copy_to(
                "/qpid-broker-j/work-override/default.json",
                config_path.as_path(),
            ),
    };
    let node = image.start().await.unwrap();

    let port = node.get_host_port_ipv4(5672).await.unwrap();
    (node, port)
}

// TODO: disable default user and add a new user
#[allow(dead_code)] // not used by all test binaries
pub async fn setup_rabbitmq_amqp10(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    // Wait for RabbitMQ to finish booting before connecting: the AMQP 1.0
    // plugin accepts TCP connections while the broker is still starting up,
    // and the client has no handshake timeout, so connecting too early hangs
    // the test until the CI job timeout.
    let wait_for = WaitFor::message_on_either_std("Server startup complete");
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
                .with_exposed_port(5672.tcp())
                .with_wait_for(wait_for)
                .with_env_var("RABBITMQ_DEFAULT_USER", username)
                .with_env_var("RABBITMQ_DEFAULT_PASS", password)
        }
        _ => GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
            .with_exposed_port(5672.tcp())
            .with_wait_for(wait_for)
            .into(),
    };
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5672).await.unwrap();
    (node, port)
}

use std::path::Path;

use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

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
pub async fn setup_rabbitmq_amqp10(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
                .with_exposed_port(5672.tcp())
                .with_wait_for(WaitFor::seconds(10))
                .with_env_var("RABBITMQ_DEFAULT_USER", username)
                .with_env_var("RABBITMQ_DEFAULT_PASS", password)
        }
        _ => GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
            .with_exposed_port(5672.tcp())
            .with_wait_for(WaitFor::seconds(10))
            .into(),
    };
    let node = image.start().await.unwrap();
    let port = node.get_host_port_ipv4(5672).await.unwrap();
    (node, port)
}

use testcontainers::{core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage};

pub async fn setup_activemq_artemis(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
                .with_env_var("ARTEMIS_USERNAME", username)
                .with_env_var("ARTEMIS_PASSWORD", password)
                .with_exposed_port(5672)
                .with_wait_for(WaitFor::seconds(5))
        }
        _ => GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
            .with_env_var("DISABLE_SECURITY", "true")
            .with_exposed_port(5672)
            .with_wait_for(WaitFor::seconds(5)),
    };

    let runner = image.start().await.unwrap();
    let port = runner.get_host_port_ipv4(5672).await.unwrap();
    (runner, port)
}

// TODO: disable default user and add a new user
pub async fn setup_rabbitmq_amqp10(
    username: Option<&str>,
    password: Option<&str>,
) -> (ContainerAsync<GenericImage>, u16) {
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
                .with_env_var("RABBITMQ_DEFAULT_USER", username)
                .with_env_var("RABBITMQ_DEFAULT_PASS", password)
                .with_exposed_port(5672)
                .with_wait_for(WaitFor::seconds(10))
        }
        _ => GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
            .with_exposed_port(5672)
            .with_wait_for(WaitFor::seconds(10)),
    };

    let runner = image.start().await.unwrap();
    let port = runner.get_host_port_ipv4(5672).await.unwrap();
    (runner, port)
}

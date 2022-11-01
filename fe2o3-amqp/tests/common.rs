use testcontainers::{clients::{Cli}, core::WaitFor, images::{self, generic::GenericImage}, Container};
use tokio::sync::OnceCell;

static DOCKER: OnceCell<Cli> = OnceCell::const_new();

pub async fn setup_activemq_artemis(username: Option<&str>, password: Option<&str>) -> (Container<'static, GenericImage>, u16) {
    let docker = DOCKER.get_or_init(|| async {Cli::default()}).await;
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
                .with_env_var("ARTEMIS_USERNAME", username)
                .with_env_var("ARTEMIS_PASSWORD", password)
                .with_exposed_port(5672)
                .with_wait_for(WaitFor::seconds(5))
        }
        _ => images::generic::GenericImage::new("docker.io/vromero/activemq-artemis", "latest")
            .with_env_var("DISABLE_SECURITY", "true")
            .with_exposed_port(5672)
            .with_wait_for(WaitFor::seconds(5)),
    };
    let node = docker.run(image);

    let port = node.get_host_port_ipv4(5672);
    (node, port)
}

// TODO: disable default user and add a new user
pub async fn setup_rabbitmq_amqp10(username: Option<&str>, password: Option<&str>) -> (Container<'static, GenericImage>, u16) {
    let docker = DOCKER.get_or_init(|| async {Cli::default()}).await;
    let image = match (username, password) {
        (Some(username), Some(password)) => {
            images::generic::GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
                .with_env_var("RABBITMQ_DEFAULT_USER", username)
                .with_env_var("RABBITMQ_DEFAULT_PASS", password)
                .with_exposed_port(5672)
                .with_wait_for(WaitFor::seconds(10))
        }
        _ => images::generic::GenericImage::new("docker.io/minghuaw/rabbitmq-amqp1.0", "latest")
            .with_exposed_port(5672)
            .with_wait_for(WaitFor::seconds(10)),
    };
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5672);
    (node, port)
}

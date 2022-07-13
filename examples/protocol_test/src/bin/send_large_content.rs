use std::{env, collections::VecDeque};

use anyhow::{anyhow, Result};
use fe2o3_amqp::{
    types::{
        messaging::Message,
        primitives::{Binary, Symbol, Value},
    },
    Connection, Sender, Session,
};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

const MEGABYTE: usize = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct SizeInMb(usize, Vec<usize>);

#[derive(Debug)]
struct TestSender {
    broker_addr: String,
    target_addr: String,
    message_iter: MessageIter,
}

impl TestSender {
    async fn run(self) -> Result<()> {
        let mut connection = Connection::open(
            "fe2o3-amqp-amqp-large-content-test-sender-connection",
            format!("amqp://{}", self.broker_addr).as_str(),
        )
        .await?;
        let mut session = Session::begin(&mut connection).await?;
        let mut sender = Sender::attach(
            &mut session,
            "fe2o3-amqp-amqp-large-content-test-sender",
            self.target_addr,
        )
        .await?;

        // let mut sender = Sender::builder()
        //     .name("fe2o3-amqp-amqp-large-content-test-sender")
        //     .target(self.target_addr)
        //     .max_message_size(1 * MEGABYTE as u64)
        //     .attach(&mut session)
        //     .await?;

        for message in self.message_iter.into_iter() {
            tracing::info!("sending new message");
            let outcome = sender.send(message).await?;
            tracing::info!(?outcome);
        }

        sender.close().await?;
        session.end().await?;
        connection.close().await?;
        Ok(())
    }
}

impl TryFrom<Vec<String>> for TestSender {
    type Error = anyhow::Error;

    fn try_from(mut value: Vec<String>) -> Result<Self, Self::Error> {
        let mut drain = value.drain(1..);

        let broker_addr = drain.next().ok_or(anyhow!("Wrong number of arguments"))?;
        let target_addr = drain.next().ok_or(anyhow!("Wrong number of arguments"))?;
        let type_name = drain.next().ok_or(anyhow!("Wrong number of arguments"))?;
        let input = drain.next().ok_or(anyhow!("Wrong number of arguments"))?;

        let message_iter = create_message_sizes(&type_name, &input)?;

        Ok(Self {
            broker_addr,
            target_addr,
            message_iter,
        })
    }
}

#[derive(Debug)]
enum MessageSizeInMb {
    Binary(usize),
    String(usize),
    Symbol(usize),
    List { total_size: usize, num_elem: usize },
    Map { total_size: usize, num_elem: usize },
}

fn create_message_sizes(type_name: &str, input: &str) -> Result<MessageIter> {
    let sizes: VecDeque<MessageSizeInMb> = match type_name {
        "binary" => {
            let sizes: Vec<usize> = from_str(input)?;
            sizes
                .into_iter()
                .map(|s| MessageSizeInMb::Binary(s))
                .collect()
        }
        "string" => {
            let sizes: Vec<usize> = from_str(input)?;
            sizes
                .into_iter()
                .map(|s| MessageSizeInMb::String(s))
                .collect()
        }
        "symbol" => {
            let sizes: Vec<usize> = from_str(input)?;
            sizes
                .into_iter()
                .map(|s| MessageSizeInMb::Symbol(s))
                .collect()
        }

        "list" => {
            let sizes: Vec<SizeInMb> = from_str(input)?;
            sizes
                .into_iter()
                .map(|s| {
                    s.1.into_iter()
                        .map(|num_elem| MessageSizeInMb::List {
                            total_size: s.0,
                            num_elem,
                        })
                        .collect::<Vec<MessageSizeInMb>>()
                })
                .flatten()
                .collect()
        }
        "map" => {
            let sizes: Vec<SizeInMb> = from_str(input)?;
            sizes
                .into_iter()
                .map(|s| {
                    s.1.into_iter()
                        .map(|num_elem| MessageSizeInMb::Map {
                            total_size: s.0,
                            num_elem,
                        })
                        .collect::<Vec<MessageSizeInMb>>()
                })
                .flatten()
                .collect()
        }
        _ => unreachable!(),
    };

    Ok(MessageIter { sizes })
}

#[derive(Debug)]
struct MessageIter {
    sizes: VecDeque<MessageSizeInMb>,
}

impl Iterator for MessageIter {
    type Item = Message<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        self.sizes.pop_front().map(|size| generate_message(size))
    }
}

fn generate_message(size: MessageSizeInMb) -> Message<Value> {
    match size {
        MessageSizeInMb::Binary(total_in_mb) => {
            let binary = Binary::from(vec![b'b'; total_in_mb * MEGABYTE]);
            Message::builder().value(Value::Binary(binary)).build()
        }
        MessageSizeInMb::String(total_in_mb) => {
            let buf = vec![b's'; total_in_mb * MEGABYTE];
            let s = String::from_utf8_lossy(&buf);
            Message::builder()
                .value(Value::String(s.to_string()))
                .build()
        }
        MessageSizeInMb::Symbol(total_in_mb) => {
            let buf = vec![b'y'; total_in_mb * MEGABYTE];
            let s = Symbol::new(String::from_utf8_lossy(&buf));
            Message::builder().value(Value::Symbol(s)).build()
        }
        MessageSizeInMb::List {
            total_size,
            num_elem,
        } => todo!(),
        MessageSizeInMb::Map {
            total_size,
            num_elem,
        } => todo!(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // .with_max_level(Level::DEBUG)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // let args: Vec<String> = env::args().collect();
    // let test_sender = TestSender::try_from(args)?;
    let message_iter = create_message_sizes("string", "[10, 10, 10, 10]")?;
    let test_sender = TestSender {
        broker_addr: "localhost:5672".to_string(),
        target_addr: "q1".to_string(),
        message_iter,
    };

    test_sender.run().await?;
    Ok(())
}

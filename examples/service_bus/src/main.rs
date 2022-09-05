fn main() {
    println!(
        r#"Please run the example with
    cargo run --bin queue_sender
OR
    cargo run --bin queue_receiver
OR
    cargo run --bin queue_dlq --features "dlq"
OR
    cargo run --bin topic_sender
OR
    cargo run --bin topic_receiver
    "#
    );
}

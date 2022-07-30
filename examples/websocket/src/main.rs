use fe2o3_amqp_ws::WebSocketStream;

#[tokio::main]
async fn main() {
    let (ws_stream, response) = WebSocketStream::connect("ws://localhost:5673").await.unwrap();

    println!("{:?}", response);

    println!("Hello, world!");
}

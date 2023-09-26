use fe2o3_amqp::{Connection, sasl_profile::SaslProfile, Session, Sender, Receiver};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    spawn_local(async {
        let local_set = tokio::task::LocalSet::new();
        local_set.run_until(async {
            use fe2o3_amqp_ws::WebSocketStream;
            use web_sys::console;
        
            // This example uses the Azure Service Bus as the AMQP broker because it provides 
            // a WebSocket endpoint.
            let hostname = "<NAMESPACE>.servicebus.windows.net";
            let sa_key_name = "<SharedAccessKeyName>";
            let sa_key_value = "<SharedAccessKey>";
        
            let ws_address = format!("wss://{sa_key_name}:{sa_key_value}@{hostname}/$servicebus/websocket");
            console::log_1(&format!("Connecting to {}", ws_address).into());
            let ws_stream = WebSocketStream::connect(ws_address).await.unwrap();
        
            let mut connection = Connection::builder()
                .container_id("wasm-client")
                .scheme("amqp")
                .hostname(hostname)
                .sasl_profile(SaslProfile::Plain {
                    username: sa_key_name.into(),
                    password: sa_key_value.into(),
                })
                .open_with_stream_on_local_set(ws_stream, &local_set)
                .await
                .unwrap();
            console::log_1(&"Connection connected".into());
        
            let mut session = Session::builder()
                .begin_on_local_set(&mut connection, &local_set).await.unwrap();
        
            console::log_1(&"Session connected".into());

            let mut sender = Sender::attach(&mut session, "wasm-sender", "q1").await.unwrap();
            sender.send("message from wasm").await.unwrap();
            sender.close().await.unwrap();
            console::log_1(&"Sender closed".into());

            let mut receiver = Receiver::attach(&mut session, "wasm-receiver", "q1").await.unwrap();
            let delivery = receiver.recv::<String>().await.unwrap();
            receiver.accept(&delivery).await.unwrap();
            console::log_1(&format!("Received: {}", delivery.body()).into());
            receiver.close().await.unwrap();

            drop(session);
            drop(connection);

            console::log_1(&"Connection and session closed".into());
        }).await;
    })
}
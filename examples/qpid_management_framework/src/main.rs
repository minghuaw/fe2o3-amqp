use fe2o3_amqp::{
    session::SessionHandle,
    types::{
        messaging::{AmqpValue, ApplicationProperties, Body, Message, Properties, Source},
        primitives::{OrderedMap, Symbol, Value},
    },
    Connection, Receiver, Sender, Session,
};

async fn create_dynamic_receiver(
    session: &mut SessionHandle<()>,
) -> Result<Receiver, Box<dyn std::error::Error>> {
    let source = Source::builder().dynamic(true).build();
    let receiver = Receiver::builder()
        .name("test-receiver")
        .source(source)
        .attach(session)
        .await?;
    Ok(receiver)
}

/// This is going to create the message body for the following content 
/// 
/// ```python
/// content={
///      '_object_id': {'_object_name': 'org.apache.qpid.broker:broker:amqp-broker'},
///      '_method_name': 'create',
///      '_arguments': {'strict': True, 'type': 'exchange', 'name': u'test-fanout', 'properties': {'exchange-type': u'fanout'}}
/// }
/// ```
fn command_message_content() -> OrderedMap<Symbol, Value> {
    let mut map = OrderedMap::new();

    // '_object_id': {'_object_name': 'org.apache.qpid.broker:broker:amqp-broker'},
    let mut object_id = OrderedMap::new();
    object_id.insert(
        Symbol::from("_object_name"),
        Value::from("org.apache.qpid.broker:broker:amqp-broker"),
    );
    map.insert(Symbol::from("_object_id"), Value::from(object_id));

    // '_method_name': 'create'
    map.insert(Symbol::from("_method_name"), Value::from("create"));

    // '_arguments': {'strict': True, 'type': 'exchange', 'name': u'test-fanout', 'properties': {'exchange-type': u'fanout'}}
    let mut arguments = OrderedMap::new();
    arguments.insert(Symbol::from("strict"), Value::from(true));
    arguments.insert(Symbol::from("type"), Value::from("exchange"));
    arguments.insert(Symbol::from("name"), Value::from("test-fanout"));
    let mut properties = OrderedMap::new();
    properties.insert(Symbol::from("exchange-type"), Value::from("fanout"));
    arguments.insert(Symbol::from("properties"), Value::from(properties));

    map.insert(Symbol::from("_arguments"), Value::from(arguments));

    map
}

/// This is going to create the message shown in the python snippet below. 
/// The python snippet is taken from https://access.redhat.com/documentation/en-us/red_hat_enterprise_mrg/3/html/messaging_programming_reference/creating_exchanges_from_an_application
/// 
/// ```python
/// Message(
///     subject='broker', 
///     reply_to='qmf.default.topic/direct.6da5bfc3-44fb-4441-b834-6c5897b9606a;{node:{type:topic}, link:{x-declare:{auto-delete:True,exclusive:True}}}', 
///     correlation_id='1', 
///     properties={
///         'qmf.opcode': '_method_request', 
///         'x-amqp-0-10.app-id': 'qmf2', 
///         'method': 'request'
///     }, 
///     content={
///         '_object_id': {'_object_name': 'org.apache.qpid.broker:broker:amqp-broker'}, 
///         '_method_name': 'create', 
///         '_arguments': {'strict': True, 'type': 'exchange', 'name': u'test-fanout', 'properties': {'exchange-type': u'fanout'}}
///     }
/// )
/// ```
fn command_message(reply_to: String) -> Message<AmqpValue<OrderedMap<Symbol, Value>>> {
    let content = command_message_content();

    Message::builder()
        // ```
        // subject='broker',
        // reply_to='qmf.default.topic/direct.6da5bfc3-44fb-4441-b834-6c5897b9606a;{node:{type:topic}, link:{x-declare:{auto-delete:True,exclusive:True}}}',
        // correlation_id='1', 
        // ```
        .properties(
            Properties::builder()
            .subject("broker")
            .reply_to(reply_to) // Change this to the address of the receiver from which you wish to receive the response
            .correlation_id(1)
            .build(),
        )
        // AMQP 0-10 message properties are mapped to message application properties in AMQP 1.0
        //
        // ```
        // properties={'qmf.opcode': '_method_request', 'x-amqp-0-10.app-id': 'qmf2', 'method': 'request'},
        // ```
        .application_properties(
            ApplicationProperties::builder()
                .insert("qmf.opcode", "_method_request")
                .insert("x-amqp-0-10.app-id", "qmf2")
                .insert("method", "request")
                .build(),
        )
        .value(content)
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = Connection::open("connection_id", "amqp://localhost:5672").await?;
    let mut session = Session::begin(&mut connection).await?;

    // Create a receiver that will be used to receive the response from QMF.
    // This doesn't have to be a dynamic receiver. This is using a dynamic receiver just for convenience.
    let mut receiver = create_dynamic_receiver(&mut session).await?;
    let source_addr = receiver
        .source()
        .as_ref()
        .and_then(|source| source.address.as_ref())
        .unwrap(); // There has to be an address if the dynamic receiver is created successfully
    println!("Source Addr {:?}", source_addr);

    // Create a sender that sends QMF command messages.
    // Although the doc says the address should be `"qmf.default.direct/broker"`, you will actually
    // get an not-found error if you use that address. The address should be `"qmf.default.direct"`.
    let mut sender = Sender::attach(&mut session, "test-sender", "qmf.default.direct").await?;

    // Prepare a QMF command message and send it.
    //
    // Definition and examples of command messages can be found here:
    // https://access.redhat.com/documentation/en-us/red_hat_enterprise_mrg/3/html/messaging_programming_reference/command_messages
    //
    // The command message is essentially a message whose body section is a map, and some entries may be nested maps.
    let request = command_message(source_addr.to_string());
    sender.send(request).await?.accepted_or("not accepted")?;
    println!("Sent");

    // The format of the response is not well documented (if documented at all).
    // So we are using `<Body<Value>>` as the type parameter to be able to receive any type of response.
    let message = receiver.recv::<Body<Value>>().await?;
    receiver.accept(&message).await?;
    println!("Received {:?}", message);

    receiver.close().await?;
    sender.close().await?;
    session.end().await?;
    connection.close().await?;
    Ok(())
}

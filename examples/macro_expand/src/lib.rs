use fe2o3_amqp::macros::AmqpContract;

#[derive(AmqpContract)]
#[amqp_contract(name = "a", code = 0x8, encoding = "list")]
struct Test {
    a: i32,
    b: bool,
}

#[derive(AmqpContract)]
struct NewType(i32);

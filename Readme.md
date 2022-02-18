# fe2o3-amqp

## Settled Transfer



### AmqpNetLite

```bash
     0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x14,
    0xc0,
    0x11,
    0xb,
    0x43,
    0x43,
    0xa0,
    0x4,
    0x0,
    0x0,
    0x0,
    0x1,
    0x43,
    0x41,
    0x40,
    0x40,
    0x40,
    0x40,
    0x40,
    0x42,
    0x0,
    0x53,
    0x75,
    0xa0,
    0xa,
    0x48,
    0x65,
    0x6c,
    0x6c,
    0x6f,
    0x20,
    0x41,
    0x4d,
    0x51,
    0x50,
```




### local


```bash
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x14,
    0xc0,
    0xb,
    0x5,
    0x43,
    0x43,
    0xa0,
    0x4,
    0x0,
    0x0,
    0x0,
    0x1,
    0x43,
    0x41,
    0x0,
    0x53,
    0x77,
    0xa1,
    0xa,
    0x48,
    0x45,
    0x4c,
    0x4c,
    0x4f,
    0x20,
    0x41,
    0x4d,
    0x51,
    0x50,
```










## Unsettled Transfer
### AmqpNetLite

```bash
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x14,
    0xc0,
    0x11,
    0xb,
    0x43,
    0x43,
    0xa0,
    0x4,
    0x0,
    0x0,
    0x0,
    0x1,
    0x43,
    0x42,
    0x40,
    0x40,
    0x40,
    0x40,
    0x40,
    0x42,
    0x0,
    0x53,
    0x75,
    0xa0,
    0xa,
    0x48,
    0x65,
    0x6c,
    0x6c,
    0x6f,
    0x20,
    0x41,
    0x4d,
    0x51,
    0x50,
```


### local




```bash
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x14,
    0xc0,
    0xa,
    0x4,
    0x43,
    0x43,
    0xa0,
    0x4,
    0x0,
    0x0,
    0x0,
    0x1,
    0x43,
    0x0,
    0x53,
    0x77,
    0xa1,
    0xa,
    0x48,
    0x45,
    0x4c,
    0x4c,
    0x4f,
    0x20,
    0x41,
    0x4d,
    0x51,
    0x50,
```

Successful SASL


```bash
Starting connection
>>> Debug: SaslProfile
>>> Debug: Transport::connect_sasl
>>> Debug: Transport::bind
>>> Debug: frame [
    0x2,
    0x1,
    0x0,
    0x0,
    0x0,
    0x53,
    0x40,
    0xc0,
    0xe,
    0x1,
    0xe0,
    0xb,
    0x1,
    0xb3,
    0x0,
    0x0,
    0x0,
    0x5,
    0x50,
    0x4c,
    0x41,
    0x49,
    0x4e,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_seq
>>> Debug: Array8
>>> Debug: ArrayAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "AMQP1.0_SYMBOL"
>>> Debug: deserialize_string
>>> Debug: ArrayAccess::next_element_seed
>>> Debug: serialize_struct
>>> Debug: frame [
    0x2,
    0x1,
    0x0,
    0x0,
    0x0,
    0x53,
    0x44,
    0xc0,
    0x3,
    0x1,
    0x50,
    0x0,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u8
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection::Builder::connect_with_stream
>>> Debug: connection::Builder::open_with_stream
>>> Debug: Transport::negotiate
>>> Debug: send_proto_header
>>> Debug: inbound_buf [
    0x41,
    0x4d,
    0x51,
    0x50,
    0x0,
    0x1,
    0x0,
    0x0,
]
>>> Debug: incoming_header ProtocolHeader { id: Amqp, major: 1, minor: 0, revision: 0 }
>>> Debug: Transport::bind
>>> Debug: Header exchanged
>>> Debug: on_outgoing_open
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x10,
    0xc0,
    0x2d,
    0x4,
    0xa1,
    0x17,
    0x54,
    0x65,
    0x73,
    0x74,
    0x41,
    0x6d,
    0x71,
    0x70,
    0x42,
    0x72,
    0x6f,
    0x6b,
    0x65,
    0x72,
    0x3a,
    0x62,
    0x62,
    0x34,
    0x64,
    0x35,
    0x64,
    0x30,
    0x64,
    0xa1,
    0x9,
    0x6c,
    0x6f,
    0x63,
    0x61,
    0x6c,
    0x68,
    0x6f,
    0x73,
    0x74,
    0x70,
    0x0,
    0x1,
    0x0,
    0x0,
    0x60,
    0x3,
    0xe7,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_string
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_string
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "MaxFrameSize"
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "ChannelMax"
>>> Debug: deserialize_u16
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection control
>>> Debug: ConectionEnginer::on_control
>>> Debug: Session::send_begin
>>> Debug: connection outgoing session frames
>>> Debug: on_outgoing_begin
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x11,
    0xc0,
    0x11,
    0x5,
    0x60,
    0x0,
    0x0,
    0x43,
    0x70,
    0x0,
    0x0,
    0x8,
    0x0,
    0x70,
    0x0,
    0x0,
    0x8,
    0x0,
    0x52,
    0x3f,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_u16
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "Handle"
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: on_incoming_begin
>>> Debug: Link::send_attach
>>> Debug: Link.local_state: Unattached
>>> Debug: Session::on_outgoing_attach
>>> Debug: connection outgoing session frames
>>> Debug: serialize_struct
>>> Debug: serialize_struct
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x12,
    0xc0,
    0x27,
    0x7,
    0xa1,
    0x12,
    0x72,
    0x75,
    0x73,
    0x74,
    0x2d,
    0x73,
    0x65,
    0x6e,
    0x64,
    0x65,
    0x72,
    0x2d,
    0x6c,
    0x69,
    0x6e,
    0x6b,
    0x2d,
    0x31,
    0x43,
    0x41,
    0x40,
    0x40,
    0x0,
    0x53,
    0x28,
    0x45,
    0x0,
    0x53,
    0x29,
    0xc0,
    0x5,
    0x1,
    0xa1,
    0x2,
    0x71,
    0x31,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_string
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "Handle"
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_string
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x13,
    0xc0,
    0x13,
    0x9,
    0x43,
    0x70,
    0x0,
    0x0,
    0x8,
    0x0,
    0x43,
    0x70,
    0x0,
    0x0,
    0x8,
    0x0,
    0x43,
    0x43,
    0x52,
    0xc8,
    0x40,
    0x42,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_newtype_struct "Handle"
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: Session::on_incoming_attach
>>> Debug: found local link
>>> Debug: Session::on_incoming_flow
>>> Debug: Link<Sender>::on_incoming_attach
>>> Debug: LinkFlowState<role::Sender>::on_incoming_flow
>>> Debug: serialize_struct
>>> Debug: SenderLink::send_transfer
>>> Debug: Session::on_outgoing_transfer
>>> Debug: connection outgoing session frames
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x15,
    0xc0,
    0x9,
    0x5,
    0x41,
    0x43,
    0x40,
    0x41,
    0x0,
    0x53,
    0x24,
    0x45,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: Session::on_incoming_disposition
>>> Debug: serialize_struct
>>> Debug: SenderLink::send_transfer
>>> Debug: Session::on_outgoing_transfer
>>> Debug: connection outgoing session frames
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x15,
    0xc0,
    0xa,
    0x5,
    0x41,
    0x52,
    0x1,
    0x40,
    0x41,
    0x0,
    0x53,
    0x24,
    0x45,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_option
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: Session::on_incoming_disposition
>>> Debug: SenderLink::send_detach
>>> Debug: SenderLink local_state: Attached
>>> Debug: Session::on_outgoing_detach
>>> Debug: connection outgoing session frames
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x16,
    0xc0,
    0x3,
    0x2,
    0x43,
    0x41,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "Handle"
>>> Debug: deserialize_u32
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_bool
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: Session::on_incoming_detach
>>> Debug: SenderLink::on_incoming_detach
>>> Debug: connection outgoing session frames
>>> Debug: on_outgoing_end
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x17,
    0x45,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: on_incoming_end
>>> Debug: Session::on_incoming_end
>>> Debug: SessionEngine exiting event_loop
>>> Debug: connection control
>>> Debug: ConectionEnginer::on_control
>>> Debug: connection control
>>> Debug: ConectionEnginer::on_control
>>> Debug: serialize_struct
>>> Debug: poll_next() resetting idle_timeout
>>> Debug: frame [
    0x2,
    0x0,
    0x0,
    0x0,
    0x0,
    0x53,
    0x18,
    0x45,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection incoming frames
>>> Debug: on_incoming_close
>>> Debug: ConnectionEngine exiting event_loop
```

Bad SASL

```bash
Starting connection
>>> Debug: SaslProfile
>>> Debug: Transport::connect_sasl
>>> Debug: Transport::bind
>>> Debug: frame [
    0x2,
    0x1,
    0x0,
    0x0,
    0x0,
    0x53,
    0x40,
    0xc0,
    0xe,
    0x1,
    0xe0,
    0xb,
    0x1,
    0xb3,
    0x0,
    0x0,
    0x0,
    0x5,
    0x50,
    0x4c,
    0x41,
    0x49,
    0x4e,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_seq
>>> Debug: Array8
>>> Debug: ArrayAccess::next_element_seed
>>> Debug: deserialize_newtype_struct "AMQP1.0_SYMBOL"
>>> Debug: deserialize_string
>>> Debug: ArrayAccess::next_element_seed
>>> Debug: serialize_struct
>>> Debug: frame [
    0x2,
    0x1,
    0x0,
    0x0,
    0x0,
    0x53,
    0x44,
    0xc0,
    0x3,
    0x1,
    0x50,
    0x0,
]
>>> Debug: deserialize_enum
>>> Debug: deserialize_identifier
>>> Debug: deserialize_identifier EnumType::None
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_struct
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_enum
>>> Debug: EnumType::Descriptor
>>> Debug: visit_enum
>>> Debug: deserialize_identifier
>>> Debug: VariantAccess::newtype_variant_seed
>>> Debug: deserialize_u64
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: deserialize_u8
>>> Debug: DescribedAccess::next_element_seed
>>> Debug: connection::Builder::connect_with_stream
>>> Debug: connection::Builder::open_with_stream
>>> Debug: Transport::negotiate
>>> Debug: send_proto_header
>>> Debug: inbound_buf [
    0x0,
    0x0,
    0x0,
    0x3a,
    0x2,
    0x0,
    0x0,
    0x0,
]
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: AmqpError { condition: NotImplemented, description: Some("Found: [0, 0, 0, 58, 2, 0, 0, 0]") }', examples\protocol_test\src\bin\sender.rs:26:10
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: process didn't exit successfully: `D:\Developments\workspace_rust\fe2o3-amqp\target\debug\sender.exe` (exit code: 101)
```
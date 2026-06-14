//! Tests for the `#[amqp_contract(multiple)]` field attribute (issue #111).
//!
//! Per the AMQP 1.0 spec (Part 1 "Types", section 1.4): "a null value and a
//! zero-length array (with a correct type for its elements) both describe an
//! absence of a value and MUST be treated as semantically identical." A field
//! marked `multiple` must therefore decode a zero-length array to the same value
//! as a null, i.e. `None`.
//!
//! These tests use the `list` encoding, which is the path every described-list
//! performative exercises at runtime. The derive also normalizes in its
//! `visit_map` path, but that is intentionally not round-trip tested here: the
//! `map` encoding does not round-trip in `serde_amqp` independently of this
//! attribute (a plain map-encoded struct fails to decode), so a map test would
//! fail for reasons unrelated to issue #111.
#![cfg(feature = "derive")]

use serde_amqp::{
    from_slice,
    primitives::{Array, Symbol},
    to_vec, DeserializeComposite, SerializeComposite,
};

#[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:multiple:list",
    code = "0x0000_0000:0x0000_0100",
    encoding = "list"
)]
struct ListMultiple {
    #[amqp_contract(multiple)]
    capabilities: Option<Array<Symbol>>,
}

/// A null and a zero-length array must decode to the same value (`None`).
#[test]
fn null_and_empty_array_decode_identically() {
    let from_null: ListMultiple =
        from_slice(&to_vec(&ListMultiple { capabilities: None }).unwrap()).unwrap();
    let from_empty: ListMultiple = from_slice(
        &to_vec(&ListMultiple {
            capabilities: Some(Array::from(Vec::<Symbol>::new())),
        })
        .unwrap(),
    )
    .unwrap();

    assert_eq!(from_null.capabilities, None);
    assert_eq!(from_empty.capabilities, None);
    assert_eq!(from_null, from_empty);
}

/// The normalization must only collapse the empty case; a populated `multiple`
/// field round-trips unchanged.
#[test]
fn non_empty_multiple_field_is_preserved() {
    let capabilities = Some(Array::from(vec![Symbol::from("FOO"), Symbol::from("BAR")]));
    let decoded: ListMultiple = from_slice(
        &to_vec(&ListMultiple {
            capabilities: capabilities.clone(),
        })
        .unwrap(),
    )
    .unwrap();

    assert_eq!(decoded.capabilities, capabilities);
}

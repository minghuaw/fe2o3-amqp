//! Part 3.4 delivery state

use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Boolean, UInt, ULong};

use crate::definitions::{Error, Fields};

#[cfg(feature = "transaction")]
use crate::transaction::Declared;
#[cfg(feature = "transaction")]
use crate::transaction::TransactionalState;

/// 3.4 Delivery State
#[derive(Debug, Clone)]
pub enum DeliveryState {
    /// 3.4.1 Received
    Received(Received),

    /// 3.4.2 Accepted
    Accepted(Accepted),

    /// 3.4.2 Rejected
    Rejected(Rejected),

    /// 3.4.4 Released
    Released(Released),

    /// 3.4.5 Modified
    Modified(Modified),

    /// 4.5.5 Declared
    #[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
    #[cfg(feature = "transaction")]
    Declared(Declared),

    /// 4.5.6 Transactional State
    #[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
    #[cfg(feature = "transaction")]
    TransactionalState(TransactionalState),
}

impl DeliveryState {
    /// Whether a state is a terminal state
    pub fn is_terminal(&self) -> bool {
        match self {
            DeliveryState::Accepted(_)
            | DeliveryState::Rejected(_)
            | DeliveryState::Released(_)
            | DeliveryState::Modified(_) => true,
            DeliveryState::Received(_) => false,

            #[cfg(feature = "transaction")]
            DeliveryState::Declared(_) => true,

            #[cfg(feature = "transaction")]
            DeliveryState::TransactionalState(_) => false,
        }
    }
}

impl AsRef<DeliveryState> for DeliveryState {
    fn as_ref(&self) -> &DeliveryState {
        self
    }
}

impl AsMut<DeliveryState> for DeliveryState {
    fn as_mut(&mut self) -> &mut DeliveryState {
        self
    }
}

mod delivery_state_impl;

/// A terminal delivery state is also referred to as Outcome
#[derive(Debug, Clone)]
pub enum Outcome {
    /// 3.4.2 Accepted
    Accepted(Accepted),

    /// 3.4.2 Rejected
    Rejected(Rejected),

    /// 3.4.4 Released
    Released(Released),

    /// 3.4.5 Modified
    Modified(Modified),

    /// 4.5.5 Declared
    #[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
    #[cfg(feature = "transaction")]
    Declared(Declared),
}

mod outcome_impl;

/// 3.4.1 Received
///
/// <type name="received" class="composite" source="list" provides="delivery-state">
/// <descriptor name="amqp:received:list" code="0x00000000:0x00000023"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:received:list",
    code = 0x0000_0000_0000_0023,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Received {
    /// <field name="section-number" type="uint" mandatory="true"/>
    pub section_number: UInt,

    /// <field name="section-offset" type="ulong" mandatory="true"/>
    pub section_offset: ULong,
}

/// 3.4.2 Accepted
/// The accepted outcome
///
/// <type name="accepted" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:accepted:list" code="0x00000000:0x00000024"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:accepted:list",
    code = 0x0000_0000_0000_0024,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Accepted {}

/// 3.4.3 Rejected
/// The rejected outcome.
///
/// <type name="rejected" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:rejected:list" code="0x00000000:0x00000025"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:rejected:list",
    code = 0x0000_0000_0000_0025,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Rejected {
    /// <field name="error" type="error"/>
    pub error: Option<Error>,
}

/// 3.4.4 Released
/// The released outcome.
/// <type name="released" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:released:list" code="0x00000000:0x00000026"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:released:list",
    code = 0x000_0000_0000_0026,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Released {}

/// 3.4.5 Modified
/// The modified outcome.
/// <type name="modified" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:modified:list" code="0x00000000:0x00000027"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:modified:list",
    code = 0x0000_0000_0000_0027,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Modified {
    /// <field name="delivery-failed" type="boolean"/>
    pub delivery_failed: Option<Boolean>,

    /// <field name="undeliverable-here" type="boolean"/>
    pub undeliverable_here: Option<Boolean>,

    /// <field name="message-annotations" type="fields"/>
    pub message_annotations: Option<Fields>,
}

#[cfg(test)]
mod tests {
    //! Test serialization and deserialization
    use serde_amqp::{de::from_slice, format_code::EncodingCodes, from_reader, ser::to_vec};

    use super::{Accepted, DeliveryState, Modified, Received, Rejected, Released};

    /* ---------------------------- // test Accepted ---------------------------- */
    #[test]
    fn test_serialize_deserialize_accepted() {
        let accepted = Accepted {};
        let buf = to_vec(&accepted).unwrap();
        let _: Accepted = from_slice(&buf).unwrap();
    }

    #[test]
    fn test_deserialize_accepted_from_list8() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x24, // descriptor code
            EncodingCodes::List8 as u8,
            0, // size
            0, // count
        ];
        let _: Accepted = from_slice(&buf).unwrap();
    }

    #[test]
    fn test_deserialize_accepted_from_list32() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x24, // descriptor code
            EncodingCodes::List32 as u8,
            0,
            0,
            0,
            0, // size
            0,
            0,
            0,
            0, // count
        ];
        let _: Accepted = from_slice(&buf).unwrap();
    }

    /* ------------------------------ test Rejected ----------------------------- */
    #[test]
    fn test_serialize_deserialize_rejected() {
        let rejected = Rejected { error: None };
        let buf = to_vec(&rejected).unwrap();
        let rejected: Rejected = from_slice(&buf).unwrap();
        assert!(rejected.error.is_none())
    }

    #[test]
    fn test_deserialize_rejected_from_list8() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x25, // descriptor code
            EncodingCodes::List8 as u8,
            0, // size
            0, // count
        ];
        let rejected: Rejected = from_slice(&buf).unwrap();
        assert!(rejected.error.is_none())
    }

    /* ------------------------------ test Released ----------------------------- */
    #[test]
    fn test_serialize_deserialize_released() {
        let released = Released {};
        let buf = to_vec(&released).unwrap();
        let _: Released = from_slice(&buf).unwrap();
    }

    /* ------------------------------ test Modified ----------------------------- */
    #[test]
    fn test_serialize_deserialize_modified() {
        let modified = Modified {
            delivery_failed: None,
            undeliverable_here: Some(true),
            message_annotations: None,
        };
        let buf = to_vec(&modified).unwrap();
        let modified2: Modified = from_slice(&buf).unwrap();
        println!("{:?}", buf);
        println!("{:?}", modified2);
    }

    /* ------------------------------ test Received ----------------------------- */
    #[test]
    fn test_serialize_deserialize_received() {
        let received = Received {
            section_number: 9,
            section_offset: 13,
        };
        let buf = to_vec(&received).unwrap();
        let received2: Received = from_slice(&buf).unwrap();
        println!("{:?}", received2);
    }

    /* --------------------------- test DeliveryState --------------------------- */

    macro_rules! assert_delivery_state {
        ($value:ident, $expecting:path) => {
            match $value {
                $expecting(_) => {}
                _ => panic!("Wrong variant"),
            }
        };
    }

    #[test]
    fn test_serialize_deserialize_delivery_state() {
        // Accepted
        let state = DeliveryState::Accepted(Accepted {});
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Accepted);

        // Rejected
        let state = DeliveryState::Rejected(Rejected { error: None });
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Rejected);

        // Released
        let state = DeliveryState::Released(Released {});
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Released);

        // Modified
        let state = DeliveryState::Modified(Modified {
            delivery_failed: None,
            undeliverable_here: Some(true),
            message_annotations: None,
        });
        let buf = to_vec(&state).unwrap();
        println!("{:?}", buf);
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Modified);
        if let DeliveryState::Modified(m) = state2 {
            assert_eq!(m.delivery_failed, None);
            assert_eq!(m.undeliverable_here, Some(true));
            assert!(m.message_annotations.is_none());
        }

        // Received
        let state = DeliveryState::Received(Received {
            section_number: 9,
            section_offset: 13,
        });
        let buf = to_vec(&state).unwrap();
        let state2 = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Received);
        if let DeliveryState::Received(r) = state2 {
            assert_eq!(r.section_number, 9);
            assert_eq!(r.section_offset, 13);
        }
    }

    #[test]
    fn test_debug() {
        // let state = DeliveryState::Modified(Modified {
        //     delivery_failed: None,
        //     undeliverable_here: Some(true),
        //     message_annotations: None,
        // });
        let state = DeliveryState::Accepted(Accepted {});
        let buf = to_vec(&state).unwrap();
        println!("{:#x?}", buf);
        let modified: DeliveryState = from_reader(&buf[..]).unwrap();
        println!("{:?}", modified);
    }
}

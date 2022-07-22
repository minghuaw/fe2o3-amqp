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

    /// Returns true if the result is [`Received`].
    pub fn is_received(&self) -> bool {
        match self {
            DeliveryState::Received(_) => true,
            DeliveryState::Accepted(_) => false,
            DeliveryState::Rejected(_) => false,
            DeliveryState::Released(_) => false,
            DeliveryState::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
            #[cfg(feature = "transaction")]
            Self::TransactionalState(_) => false,
        }
    }

    /// Returns true if the result is [`Accepted`].
    ///
    /// A transactional state will always return false
    pub fn is_accepted(&self) -> bool {
        match self {
            DeliveryState::Received(_) => false,
            DeliveryState::Accepted(_) => true,
            DeliveryState::Rejected(_) => false,
            DeliveryState::Released(_) => false,
            DeliveryState::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
            #[cfg(feature = "transaction")]
            Self::TransactionalState(_) => false,
        }
    }

    /// Returns true if the result is [`Rejected`].
    ///
    /// A transactional state will always return false
    pub fn is_rejected(&self) -> bool {
        match self {
            DeliveryState::Received(_) => false,
            DeliveryState::Accepted(_) => false,
            DeliveryState::Rejected(_) => true,
            DeliveryState::Released(_) => false,
            DeliveryState::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
            #[cfg(feature = "transaction")]
            Self::TransactionalState(_) => false,
        }
    }

    /// Returns true if the result is [`Released`].
    ///
    /// A transactional state will always return false
    pub fn is_released(&self) -> bool {
        match self {
            DeliveryState::Received(_) => false,
            DeliveryState::Accepted(_) => false,
            DeliveryState::Rejected(_) => false,
            DeliveryState::Released(_) => true,
            DeliveryState::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
            #[cfg(feature = "transaction")]
            Self::TransactionalState(_) => false,
        }
    }

    /// Returns true if the result is [`Modified`].
    ///
    /// A transactional state will always return false
    pub fn is_modified(&self) -> bool {
        match self {
            DeliveryState::Received(_) => false,
            DeliveryState::Accepted(_) => false,
            DeliveryState::Rejected(_) => false,
            DeliveryState::Released(_) => false,
            DeliveryState::Modified(_) => true,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
            #[cfg(feature = "transaction")]
            Self::TransactionalState(_) => false,
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Received, E>`,
    /// mapping Received(received) to Ok(received) and other variants to Err(err).
    pub fn received_or<E>(self, err: E) -> Result<Received, E> {
        match self {
            Self::Received(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Accepted, E>`,
    /// mapping Accepted(accepted) to Ok(accepted) and other variants to Err(err).
    pub fn accepted_or<E>(self, err: E) -> Result<Accepted, E> {
        match self {
            Self::Accepted(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Rejected, E>`,
    /// mapping Rejected(rejected) to Ok(rejected) and other variants to Err(err).
    pub fn rejected_or<E>(self, err: E) -> Result<Rejected, E> {
        match self {
            Self::Rejected(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Released, E>`,
    /// mapping Released(released) to Ok(released) and other variants to Err(err).
    pub fn released_or<E>(self, err: E) -> Result<Released, E> {
        match self {
            Self::Released(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Modified, E>`,
    /// mapping Modified(modified) to Ok(modified) and other variants to Err(err).
    pub fn modified_or<E>(self, err: E) -> Result<Modified, E> {
        match self {
            Self::Modified(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Declared, E>`,
    /// mapping Declared(declared) to Ok(declared) and other variants to Err(err).
    #[cfg(feature = "transaction")]
    pub fn declared_or<E>(self, err: E) -> Result<Declared, E> {
        match self {
            Self::Declared(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Accepted, E>`,
    /// mapping Accepted(accepted) to Ok(accepted) and other variants to Err(err).
    pub fn accepted_or_else<E, F>(self, op: F) -> Result<Accepted, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Accepted(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Rejected, E>`,
    /// mapping Rejected(rejected) to Ok(rejected) and other variants to Err(err).
    pub fn rejected_or_else<E, F>(self, op: F) -> Result<Rejected, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Rejected(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Released, E>`,
    /// mapping Released(released) to Ok(released) and other variants to Err(err).
    pub fn released_or_else<E, F>(self, op: F) -> Result<Released, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Released(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Modified, E>`,
    /// mapping Modified(modified) to Ok(modified) and other variants to Err(err).
    pub fn modified_or_else<E, F>(self, op: F) -> Result<Modified, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Modified(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Declared, E>`,
    /// mapping Declared(declared) to Ok(declared) and other variants to Err(err).
    #[cfg(feature = "transaction")]
    pub fn declared_or_else<E, F>(self, op: F) -> Result<Declared, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Declared(value) => Ok(value),
            _ => Err(op(self)),
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

impl Outcome {
    /// Returns true if the result is [`Accepted`].
    pub fn is_accepted(&self) -> bool {
        match self {
            Self::Accepted(_) => true,
            Self::Rejected(_) => false,
            Self::Released(_) => false,
            Self::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
        }
    }

    /// Returns true if the result is [`Rejected`].
    pub fn is_rejected(&self) -> bool {
        match self {
            Self::Accepted(_) => false,
            Self::Rejected(_) => true,
            Self::Released(_) => false,
            Self::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
        }
    }

    /// Returns true if the result is [`Released`].
    pub fn is_released(&self) -> bool {
        match self {
            Self::Accepted(_) => false,
            Self::Rejected(_) => false,
            Self::Released(_) => true,
            Self::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
        }
    }

    /// Returns true if the result is [`Modified`].
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Accepted(_) => false,
            Self::Rejected(_) => false,
            Self::Released(_) => false,
            Self::Modified(_) => true,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => false,
        }
    }

    /// Returns true if the result is [`Declared`].
    pub fn is_declared(&self) -> bool {
        match self {
            Self::Accepted(_) => false,
            Self::Rejected(_) => false,
            Self::Released(_) => false,
            Self::Modified(_) => false,
            #[cfg(feature = "transaction")]
            Self::Declared(_) => true,
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Accepted, E>`,
    /// mapping Accepted(accepted) to Ok(accepted) and other variants to Err(err).
    pub fn accepted_or<E>(self, err: E) -> Result<Accepted, E> {
        match self {
            Self::Accepted(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Rejected, E>`,
    /// mapping Rejected(rejected) to Ok(rejected) and other variants to Err(err).
    pub fn rejected_or<E>(self, err: E) -> Result<Rejected, E> {
        match self {
            Self::Rejected(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Released, E>`,
    /// mapping Released(released) to Ok(released) and other variants to Err(err).
    pub fn released_or<E>(self, err: E) -> Result<Released, E> {
        match self {
            Self::Released(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Modified, E>`,
    /// mapping Modified(modified) to Ok(modified) and other variants to Err(err).
    pub fn modified_or<E>(self, err: E) -> Result<Modified, E> {
        match self {
            Self::Modified(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Declared, E>`,
    /// mapping Declared(declared) to Ok(declared) and other variants to Err(err).
    #[cfg(feature = "transaction")]
    pub fn declared_or<E>(self, err: E) -> Result<Declared, E> {
        match self {
            Self::Declared(value) => Ok(value),
            _ => Err(err),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Accepted, E>`,
    /// mapping Accepted(accepted) to Ok(accepted) and other variants to Err(err).
    pub fn accepted_or_else<E, F>(self, op: F) -> Result<Accepted, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Accepted(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Rejected, E>`,
    /// mapping Rejected(rejected) to Ok(rejected) and other variants to Err(err).
    pub fn rejected_or_else<E, F>(self, op: F) -> Result<Rejected, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Rejected(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Released, E>`,
    /// mapping Released(released) to Ok(released) and other variants to Err(err).
    pub fn released_or_else<E, F>(self, op: F) -> Result<Released, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Released(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Modified, E>`,
    /// mapping Modified(modified) to Ok(modified) and other variants to Err(err).
    pub fn modified_or_else<E, F>(self, op: F) -> Result<Modified, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Modified(value) => Ok(value),
            _ => Err(op(self)),
        }
    }

    /// Transforms the [`DeliveryState`] into a `Result<Declared, E>`,
    /// mapping Declared(declared) to Ok(declared) and other variants to Err(err).
    #[cfg(feature = "transaction")]
    pub fn declared_or_else<E, F>(self, op: F) -> Result<Declared, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            Self::Declared(value) => Ok(value),
            _ => Err(op(self)),
        }
    }
}

mod outcome_impl;

impl From<Outcome> for DeliveryState {
    fn from(value: Outcome) -> Self {
        match value {
            Outcome::Accepted(val) => Self::Accepted(val),
            Outcome::Rejected(val) => Self::Rejected(val),
            Outcome::Released(val) => Self::Released(val),
            Outcome::Modified(val) => Self::Modified(val),

            #[cfg(feature = "transaction")]
            Outcome::Declared(val) => Self::Declared(val),
        }
    }
}

/// 3.4.1 Received
///
/// <type name="received" class="composite" source="list" provides="delivery-state">
/// <descriptor name="amqp:received:list" code="0x00000000:0x00000023"/>
/// </type>
#[derive(
    Debug, Clone, DeserializeComposite, SerializeComposite, PartialEq, Eq, PartialOrd, Ord,
)]
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

impl From<Received> for DeliveryState {
    fn from(value: Received) -> Self {
        Self::Received(value)
    }
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

impl From<Accepted> for DeliveryState {
    fn from(value: Accepted) -> Self {
        Self::Accepted(value)
    }
}

impl From<Accepted> for Outcome {
    fn from(value: Accepted) -> Self {
        Self::Accepted(value)
    }
}

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

impl From<Rejected> for DeliveryState {
    fn from(value: Rejected) -> Self {
        Self::Rejected(value)
    }
}

impl From<Rejected> for Outcome {
    fn from(value: Rejected) -> Self {
        Self::Rejected(value)
    }
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

impl From<Released> for DeliveryState {
    fn from(value: Released) -> Self {
        Self::Released(value)
    }
}

impl From<Released> for Outcome {
    fn from(value: Released) -> Self {
        Self::Released(value)
    }
}

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

impl From<Modified> for DeliveryState {
    fn from(value: Modified) -> Self {
        Self::Modified(value)
    }
}

impl From<Modified> for Outcome {
    fn from(value: Modified) -> Self {
        Self::Modified(value)
    }
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

    #[test]
    fn test_compare_received() {
        let smaller = Received {
            section_number: 0,
            section_offset: 10,
        };
        let larger = Received {
            section_number: 0,
            section_offset: 11,
        };
        assert!(smaller < larger);

        let smaller = Received {
            section_number: 0,
            section_offset: 11,
        };
        let larger = Received {
            section_number: 1,
            section_offset: 11,
        };
        assert!(smaller < larger);

        let smaller = Received {
            section_number: 0,
            section_offset: 11,
        };
        let larger = Received {
            section_number: 1,
            section_offset: 1,
        };
        assert!(smaller < larger);

        let smaller = Received {
            section_number: 1,
            section_offset: 11,
        };
        let larger = Received {
            section_number: 1,
            section_offset: 11,
        };
        assert!(smaller == larger);
    }
}

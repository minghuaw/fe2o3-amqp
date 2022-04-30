#![deny(missing_docs, missing_debug_implementations)]

//! Implements AMQP1.0 data types as defined in the core [specification](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-overview-v1.0-os.html).
//! 
//! Feature flags
//! 
//! Please note that `Performative` will require both `"transport"` and `"messaging"` feature flags 
//! enabled.
//! 
//! - `"primitive"`: enables the primitive types defined in part 1.6 in the core specification.
//! - `"transport"`: enables most of the types defined in part 2.4, 2.5, and 2.8 of the core specifiction.
//! - `"messaging"`: enables the types defined in part 2.7 and part 3 defined in the core specification
//! - `"transaction"`: enables the types defined in part 4.5 of the core specification
//! - `"security"`: enables the types defined in part 5 of the core specifiction.
//! 
//! ```toml
//! default = [
//!     "primitive",
//!     "transport",
//!     "messaging",
//!     "security",
//! ]
//! ```

// List of archetypes:
// 
// 1. "frame"
// 2. "error-condition"
// 3. "section"
// 4. "message-id"
// 5. "address"
// 6. "delivery-state"
// 7. "outcome"
// 8. "source"
// 9. "target"
// 10. "distribution-mode"
// 11. "lifetime-policy"
// 12. "txn-id"
// 13. "txn-capability"
// 14. "sasl-frame"
// 15. 
// 
// List of restricted types:
// 
// 1. "role"
// 2. "sender-settle-mode"
// 3. "receiver-settle-mode"
// 4. "handle"
// 5. "seconds"
// 6. "milliseconds"
// 7. "delivery-tag"
// 8. "delivery-number"
// 9. "transfer-number"
// 10. "sequence-no"
// 11. "message-format"
// 12. "ietf-language-tag"
// 13. "fields"
// 14. "amqp-error"
// 15. "connection-error"
// 16. "session-error"
// 17. "link-error"
// 18. "delivery-annotations"
// 19. "message-annotations"
// 20. "application-properties"
// 21. "data"
// 22. "amqp-sequence"
// 23. "amqp-value"
// 24. "footer"
// 25. "annotations"
// 26. "message-id-ulong"
// 27. "message-id-uuid"
// 28. "message-id-binary"
// 31. "message-id-string"
// 32. "address-string"
// 33. "terminus-durability"
// 34. "terminus-expiry-policy"
// 35. "std-dist-mode"
// 36. "filter-set"
// 37. "node-properties"
// 38. "transaction-id"
// 39. "txn-capability"
// 40. "transaction-error"
// 41. "sasl-code"
// 

#[cfg(feature = "primitive")]
pub mod primitives;

#[cfg(feature = "transport")]
pub mod definitions;

#[cfg(feature = "messaging")]
pub mod messaging;

#[cfg(all(feature = "transport", feature = "messaging"))]
pub mod performatives;

#[cfg(feature = "security")]
pub mod sasl;

#[cfg(feature = "transport")]
pub mod states;

#[cfg(feature = "transaction")]
pub mod transaction;

use serde::{Deserialize, Serialize};
use serde_amqp::{
    primitives::{Binary, Symbol},
    DeserializeComposite, SerializeComposite,
};

/// 5.3.3.1 SASL Mechanisms
/// Advertise available sasl mechanisms.
/// <type name="sasl-mechanisms" class="composite" source="list" provides="sasl-frame">
///     <descriptor name="amqp:sasl-mechanisms:list" code="0x00000000:0x00000040"/>
///     <field name="sasl-server-mechanisms" type="symbol" multiple="true" mandatory="true"/>
/// </type>
/// Advertises the available SASL mechanisms that can be used for authentication.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:sasl-mechanisms:list",
    code = 0x0000_0000_0000_0040,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct SaslMechanisms {
    /// sasl-server-mechanisms supported sasl mechanisms
    ///
    /// A list of the sasl security mechanisms supported by the sending peer. It is invalid for
    /// this list to be null or empty. If the sending peer does not require its partner to
    /// authenticate with it, then it SHOULD send a list of one element with its value as the
    /// SASL mechanism ANONYMOUS. The server mechanisms are ordered in decreasing level of
    /// preference.
    pub sasl_server_mechanisms: Vec<Symbol>,
}

/// 5.3.3.2 SASL Init
/// Initiate sasl exchange.
/// <type name="sasl-init" class="composite" source="list" provides="sasl-frame">
///     <descriptor name="amqp:sasl-init:list" code="0x00000000:0x00000041"/>
///     <field name="mechanism" type="symbol" mandatory="true"/>
///     <field name="initial-response" type="binary"/>
///     <field name="hostname" type="string"/>
/// </type>
/// Selects the sasl mechanism and provides the initial response if needed.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:sasl-init:list",
    code = 0x0000_0000_0000_0041,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct SaslInit {
    pub mechanism: Symbol,
    pub initial_response: Option<Binary>,
    pub hostname: Option<String>,
}

/// 5.3.3.3 SASL Challenge
/// Security mechanism challenge.
/// <type name="sasl-challenge" class="composite" source="list" provides="sasl-frame">
///     <descriptor name="amqp:sasl-challenge:list" code="0x00000000:0x00000042"/>
///     <field name="challenge" type="binary" mandatory="true"/>
/// </type>
/// Send the SASL challenge data as defined by the SASL specification.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:sasl-challenge:list",
    code = 0x0000_0000_0000_0042,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct SaslChallenge {
    pub challenge: Binary,
}

// 5.3.3.4 SASL Response
// Security mechanism response.
// <type name="sasl-response" class="composite" source="list" provides="sasl-frame">
//     <descriptor name="amqp:sasl-response:list" code="0x00000000:0x00000043"/>
//     <field name="response" type="binary" mandatory="true"/>
// </type>
// Send the SASL response data as defined by the SASL specification.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:sasl-response:list",
    code = 0x0000_0000_0000_0043,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct SaslResponse {
    pub response: Binary,
}

/// 5.3.3.5 SASL Outcome
/// Indicates the outcome of the sasl dialog.
/// <type name="sasl-outcome" class="composite" source="list" provides="sasl-frame">
///     <descriptor name="amqp:sasl-outcome:list" code="0x00000000:0x00000044"/>
///     <field name="code" type="sasl-code" mandatory="true"/>
///     <field name="additional-data" type="binary"/>
/// </type>
/// This frame indicates the outcome of the SASL dialog. Upon successful completion of the SASL
/// dialog the security layer has been established, and the peers MUST exchange protocol headers
/// to either start a nested security layer, or to establish the AMQP connection.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:sasl-outcome:list",
    code = 0x0000_0000_0000_0044,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct SaslOutcome {
    pub code: SaslCode,
    pub additional_data: Option<Binary>,
}

/// 5.3.3.6 SASL Code
/// Codes to indicate the outcome of the sasl dialog.
/// <type name="sasl-code" class="restricted" source="ubyte">
///     <choice name="ok" value="0"/>
///     <choice name="auth" value="1"/>
///     <choice name="sys" value="2"/>
///     <choice name="sys-perm" value="3"/>
///     <choice name="sys-temp" value="4"/>
/// </type>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum SaslCode {
    Ok = 0u8,
    Auth = 1,
    Sys = 2,
    SysPerm = 3,
    SysTemp = 4,
}

pub mod constant {
    pub const SASL_MAJOR: u8 = 1;
    pub const SASL_MINOR: u8 = 0; // minor protocol version
    pub const SASL_REVISION: u8 = 0; // protocol revision.
}

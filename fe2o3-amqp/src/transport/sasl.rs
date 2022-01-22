use fe2o3_amqp_types::sasl::{SaslMechanisms, SaslInit, SaslChallenge, SaslResponse, SaslOutcome};

pub struct Frame {
    pub body: FrameBody
}

/// TODO: impl Serialize and Deserialize
pub enum FrameBody {
    Mechanisms(SaslMechanisms),
    Init(SaslInit),
    Challenge(SaslChallenge),
    Response(SaslResponse),
    Outcome(SaslOutcome),
}
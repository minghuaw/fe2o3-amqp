pub enum SaslProfile {
    Anonymous,
    Plain {
        user: String,
        password: String,
    }
}
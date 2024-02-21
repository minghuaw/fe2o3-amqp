cfg_scram! {
    pub use super::scram::ScramErrorKind;

    cfg_acceptor!{
        pub use super::scram::ServerScramErrorKind;
    }
}

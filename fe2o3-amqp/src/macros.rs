#![allow(unused_macros)]

macro_rules! cfg_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(target_arch = "wasm32")))]
            #[cfg(target_arch = "wasm32")]
            $item
        )*
    };
}

macro_rules! cfg_not_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(not(target_arch = "wasm32"))))]
            #[cfg(not(target_arch = "wasm32"))]
            $item
        )*
    }
}

/// Acceptor/server is not supported in wasm32 targets
macro_rules! cfg_acceptor {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(feature = "acceptor")]
            $item
        )*
    }
}

macro_rules! cfg_transaction {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
            #[cfg(feature = "transaction")]
            $item
        )*
    }
}

macro_rules! cfg_not_transaction {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(not(feature = "transaction"))))]
            #[cfg(not(feature = "transaction"))]
            $item
        )*
    }
}

macro_rules! cfg_native_tls {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
            #[cfg(feature = "native-tls")]
            $item
        )*
    }
}

macro_rules! cfg_rustls {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
            #[cfg(feature = "rustls")]
            $item
        )*
    }
}

macro_rules! cfg_scram {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
            #[cfg(feature = "scram")]
            $item
        )*
    }
}

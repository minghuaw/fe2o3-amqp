macro_rules! cfg_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg(target_arch = "wasm32")]
            $item
        )*
    };
}

macro_rules! cfg_not_wasm32 {
    ($($item:item)*) => {
        $(
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

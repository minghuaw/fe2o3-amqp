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

pub extern crate derive_more;
pub extern crate flaken;

#[macro_export]
macro_rules! id_new_type {
    ($type_name:ident) => {
        #[derive(
            Debug,
            ::serde::Serialize,
            ::serde::Deserialize,
            PartialEq,
            PartialOrd,
            Eq,
            Hash,
            Clone,
            Copy,
            $crate::id_macro::derive_more::From,
            $crate::id_macro::derive_more::Display,
            $crate::id_macro::derive_more::FromStr,
        )]
        pub struct $type_name(u64);

        impl $type_name {
            pub fn next_id() -> Self {
                use flaken::Flaken;
                use std::sync::{Mutex, OnceLock};
                use $crate::id_macro::flaken;
                static USER_ID_GENERATOR: OnceLock<Mutex<Flaken>> = OnceLock::new();
                let f = USER_ID_GENERATOR.get_or_init(|| {
                    let ip = utils::process::get_local_ip_u32();
                    let f = flaken::Flaken::default();
                    let f = f.node(ip as u64);
                    Mutex::new(f)
                });
                let mut lock = f.lock().unwrap();
                Self(lock.next())
            }
        }
    };
}

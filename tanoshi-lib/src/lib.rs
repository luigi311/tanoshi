#![crate_name = "tanoshi_lib"]

pub mod extensions;
pub mod data;
pub mod shim;
pub mod prelude;
pub mod error;

/// This is used to ensure both application and extension use the same version
pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Rust doesn't have stable ABI, this is used to ensure `rustc` version is match
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

macro_rules! register_extension {
    ($t:ty) => {
        thread_local!{
            
        }
    };
}
#![crate_name = "tanoshi_lib"]

pub mod data;
pub mod error;
pub mod extensions;
pub mod prelude;
pub mod util;

/// This is used to ensure both application and extension use the same version
pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Rust doesn't have stable ABI, this is used to ensure `rustc` version is match
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

#[macro_export]
macro_rules! register_extension {
    ($t:ty) => {
        thread_local! {
            static EXT: std::cell::RefCell<$t> = std::cell::RefCell::new(Default::default());
        }

        fn main() {
            
        }

        #[no_mangle]
        fn detail() {
            let res = EXT.with(|ext| ext.borrow_mut().detail());
            $crate::util::write_object(&res);
        }

        #[no_mangle]
        fn get_manga_list() {
            let res = EXT.with(|ext| ext.borrow_mut().get_manga_list($crate::util::read_object()));
            $crate::util::write_object(&res);
        }

        #[no_mangle]
        fn get_manga_info() {
            let res = EXT.with(|ext| ext.borrow_mut().get_manga_info($crate::util::read_object()));
            $crate::util::write_object(&res);
        }

        #[no_mangle]
        fn get_chapters() {
            let res = EXT.with(|ext| ext.borrow_mut().get_chapters($crate::util::read_object()));
            $crate::util::write_object(&res);
        }

        #[no_mangle]
        fn get_pages() {
            let res = EXT.with(|ext| ext.borrow_mut().get_pages($crate::util::read_object()));
            $crate::util::write_object(&res);
        }

        // fn get_page(&self, url: &String) -> Result<Vec<u8>, Box<dyn Error>> {
        // }

        // fn login(&self, _: SourceLogin) -> Result<SourceLoginResult, Box<dyn Error>> {
        // }
    };
}

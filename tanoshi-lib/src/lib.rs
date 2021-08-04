pub mod data;
pub mod extensions;
pub mod prelude;
pub mod shim;

/// This is used to ensure both application and extension use the same version
pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_export]
macro_rules! register_extension {
    ($t:ty) => {
        thread_local! {
            static EXT: std::cell::RefCell<$t> = std::cell::RefCell::new(Default::default());
        }

        fn main() {}

        #[no_mangle]
        fn detail() {
            let res = EXT.with(|ext| ext.borrow_mut().detail());
            $crate::shim::write_object(&res);
        }

        #[no_mangle]
        fn filters() {
            let res = EXT.with(|ext| ext.borrow_mut().filters());
            $crate::shim::write_object(&res);
        }

        #[no_mangle]
        fn get_manga_list() {
            let res = EXT.with(|ext| ext.borrow_mut().get_manga_list($crate::shim::read_object()));
            $crate::shim::write_object(&res);
        }

        #[no_mangle]
        fn get_manga_info() {
            let res = EXT.with(|ext| ext.borrow_mut().get_manga_info($crate::shim::read_object()));
            $crate::shim::write_object(&res);
        }

        #[no_mangle]
        fn get_chapters() {
            let res = EXT.with(|ext| ext.borrow_mut().get_chapters($crate::shim::read_object()));
            $crate::shim::write_object(&res);
        }

        #[no_mangle]
        fn get_pages() {
            let res = EXT.with(|ext| ext.borrow_mut().get_pages($crate::shim::read_object()));
            $crate::shim::write_object(&res);
        }

        // fn get_page(&self, url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // }

        // fn login(&self, _: SourceLogin) -> Result<SourceLoginResult, Box<dyn Error>> {
        // }
    };
}

#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key.to_string(), $val.to_string()); )*
         map
    }}
}


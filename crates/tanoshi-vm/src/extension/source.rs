use std::sync::Arc;

use libloading::Library;
use once_cell::sync::OnceCell;
use tanoshi_lib::prelude::Extension;

pub struct Source {
    pub(crate) extension: OnceCell<Box<dyn Extension>>,
    #[allow(dead_code)]
    pub(crate) lib: Option<Arc<Library>>,
}

impl Source {
    pub fn new(lib: Arc<Library>) -> Source {
        Source {
            lib: Some(lib),
            extension: OnceCell::new(),
        }
    }

    pub fn from(extension: Box<dyn Extension>) -> Self {
        Self {
            lib: None,
            extension: OnceCell::from(extension),
        }
    }
}

impl tanoshi_lib::extensions::PluginRegistrar for Source {
    fn register_function(&mut self, extension: Box<dyn Extension>) {
        self.extension
            .set(extension)
            .map_err(|_| "extension already initiated")
            .unwrap();
    }
}

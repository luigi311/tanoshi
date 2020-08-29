mod manga_list;
pub use manga_list::MangaList;

mod page_list;
pub use page_list::PageList;

mod page;
pub use page::Page;

mod manga;
pub use self::manga::Manga;

mod navigation_bar;
pub use self::navigation_bar::NavigationBar;

mod spinner;
pub use self::spinner::Spinner;

pub mod model;

mod filter;
pub use filter::Filter;

mod pager;
pub use pager::Pager;

mod reader_toolbar;
pub use reader_toolbar::ReaderToolbar;

mod reader_seekbar;
pub use reader_seekbar::ReaderSeekbar;

mod webtoon;
pub use webtoon::Webtoon;

mod toast;
pub use toast::{Toast, Type as ToastType};

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use yew::{Component, ComponentLink};

pub struct WeakComponentLink<COMP: Component>(Rc<RefCell<Option<ComponentLink<COMP>>>>);

impl<COMP: Component> Default for WeakComponentLink<COMP> {
    fn default() -> Self {
        WeakComponentLink(Rc::new(RefCell::new(None)))
    }
}

impl<COMP: Component> Deref for WeakComponentLink<COMP> {
    type Target = Rc<RefCell<Option<ComponentLink<COMP>>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<COMP: Component> Clone for WeakComponentLink<COMP> {
    fn clone(&self) -> Self {
        WeakComponentLink(self.0.clone())
    }
}

impl<COMP: Component> PartialEq for WeakComponentLink<COMP> {
    fn eq(&self, other: &WeakComponentLink<COMP>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

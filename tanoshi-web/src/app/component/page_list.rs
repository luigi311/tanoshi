use crate::app::component::page::{Page, Props as PageProps};
use yew::html::{ChildrenRenderer, Component, ComponentLink};
use yew::prelude::*;
use yew::virtual_dom::{VChild, VComp, VNode};

use super::model::{PageRendering, ReadingDirection};
use super::{Touched, WeakComponentLink};

#[derive(Clone, PartialEq)]
pub enum Variants {
    PageItem(<Page as Component>::Properties),
}

impl From<PageProps> for Variants {
    fn from(props: PageProps) -> Self {
        Variants::PageItem(props)
    }
}

#[derive(Clone, PartialEq)]
pub struct PageVariant {
    props: Variants,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub children: ChildrenRenderer<PageVariant>,
    pub weak_link: WeakComponentLink<PageList>,
    pub page_rendering: PageRendering,
    pub reading_direction: ReadingDirection,
    pub current_page: usize,
}
pub struct PageList {
    props: Props,
    class: Vec<&'static str>,
}

pub enum Msg {}

impl Component for PageList {
    type Message = Msg;
    type Properties = Props;
    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        *props.weak_link.borrow_mut() = Some(link);

        let reading_direction = match props.reading_direction {
            ReadingDirection::LeftToRight => "",
            ReadingDirection::RightToLeft => "flex-row-reverse",
        };

        let page_rendering = match props.page_rendering {
            PageRendering::LongStrip => "flex-col",
            _ => "h-screen",
        };

        let class = vec![
            "flex",
            "justify-center",
            "overflow-auto",
            reading_direction,
            page_rendering,
        ];

        PageList { props, class }
    }
    fn update(&mut self, _: <Self as yew::html::Component>::Message) -> bool {
        true
    }
    fn change(&mut self, props: Self::Properties) -> bool {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }
    fn view(&self) -> Html {
        html! {
            <div class={self.class.clone()}>
                {self.view_page()}
            </div>
        }
    }
}

impl PageList {
    fn view_page(&self) -> Html {
        html! {{
            for self.props.children.iter().map(|mut m|{
                if let Variants::PageItem(ref mut props) = m.props {
                    match self.props.page_rendering {
                        PageRendering::SinglePage => {
                            props.hidden = props.id != self.props.current_page;
                        }
                        PageRendering::DoublePage => {
                            props.hidden = (props.id != self.props.current_page) && (props.id != self.props.current_page + 1);
                        }
                        PageRendering::LongStrip => {
                            props.hidden = false;
                        }
                    }
                }
                m
            })
        }}
    }
}

impl<CHILD> From<VChild<CHILD>> for PageVariant
where
    CHILD: Component,
    CHILD::Properties: Into<Variants>,
{
    fn from(vchild: VChild<CHILD>) -> Self {
        PageVariant {
            props: vchild.props.into(),
        }
    }
}

impl Into<VNode> for PageVariant {
    fn into(self) -> VNode {
        match self.props {
            Variants::PageItem(props) => VComp::new::<Page>(props, NodeRef::default(), None).into(),
        }
    }
}

use serde::{Deserialize, Serialize};
use wasm_bindgen::__rt::core::time::Duration;
use yew::format::{Json, Nothing, Text};
use yew::services::fetch::{FetchTask, Request, Response};
use yew::services::storage::Area;
use yew::services::{FetchService, StorageService, Task, TimeoutService};
use yew::{html, Bridge, Bridged, Component, ComponentLink, Html, Properties, ShouldRender};

use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub is_active: bool,
}

pub struct Spinner {
    is_active: bool,
}

pub enum Msg {
    Noop,
}

impl Component for Spinner {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Spinner {
            is_active: props.is_active,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.is_active != props.is_active {
            self.is_active = props.is_active;
            return true;
        }
        false
    }

    fn view(&self) -> Html {
        html! {
                <div class={if !self.is_active {"hidden"} else {"w-full h-full fixed block top-0 left-0 bg-white opacity-75 z-10"}}>
                  <span class="text-green-500 opacity-75 top-1/2 my-0 mx-auto block relative w-20 h-20" style="top: 50%;">
                    <i class="fas fa-circle-notch fa-spin fa-5x"></i>
                  </span>
                </div>
        }
    }
}

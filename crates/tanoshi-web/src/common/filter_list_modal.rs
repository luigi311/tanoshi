use std::{collections::BTreeMap, rc::Rc};

use crate::common::{Input, InputType};
use dominator::{clone, events, html, svg, with_node, Dom};
use futures_signals::signal::{Mutable, ReadOnlyMutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use futures_signals::{signal::Signal, signal_map::MutableBTreeMap, signal_vec};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, HtmlInputElement, HtmlSelectElement};

#[derive(Deserialize, Serialize)]
pub struct FilterListModal {
    #[serde(skip, default = "default_first_render")]
    first_render: Mutable<bool>,
    pub filter_list: MutableVec<Input>,
    #[serde(skip)]
    show: Mutable<bool>,
    collapse: MutableBTreeMap<usize, bool>,
}

fn default_first_render() -> Mutable<bool> {
    Mutable::new(true)
}

impl FilterListModal {
    pub fn new() -> Self {
        Self {
            first_render: Mutable::new(true),
            filter_list: MutableVec::new(),
            show: Mutable::new(false),
            collapse: MutableBTreeMap::new(),
        }
    }

    pub fn set(&self, filter_list: Vec<Input>) {
        let collapse_map: BTreeMap<usize, bool> = filter_list
            .iter()
            .enumerate()
            .filter_map(|(index, input)| {
                matches!(input, Input::Sort { .. } | Input::Group { .. }).then(|| (index, false))
            })
            .collect();
        info!("{:?}", collapse_map);
        self.collapse.lock_mut().replace_cloned(collapse_map);
        self.filter_list.lock_mut().replace_cloned(filter_list);
    }

    pub fn show(&self) {
        self.show.set(true);
        self.first_render.replace_with(|_| false);
    }

    fn collapse_signal(&self, index: usize) -> impl Signal<Item = bool> {
        self.collapse
            .entries_cloned()
            .filter_map(move |(i, state)| (index == i && state).then(|| 1))
            .sum()
            .map(|sum| sum == 1)
    }

    fn render_collapsible_header(
        filter_list_modal: Rc<Self>,
        index: ReadOnlyMutable<Option<usize>>,
        name: String,
    ) -> Dom {
        html!("div", {
            .style("display", "flex")
            .style("justify-content", "space-between")
            .event(clone!(filter_list_modal, index => move |_: events::Click| {
                if let Some(index) = index.get() {
                    let state = if let Some(state) = filter_list_modal.collapse.lock_ref().get(&index).cloned() {
                        state
                    } else {
                        return;
                    };
                    filter_list_modal.collapse.lock_mut().insert(index, !state);
                }
            }))
            .children(&mut [
                html!("label", {
                    .text(&name)
                }),
                svg!("svg", {
                    .attribute("xmlns", "http://www.w3.org/2000/svg")
                    .attribute("fill", "currentColor")
                    .attribute("viewBox", "0 0 20 20")
                    .class("icon")
                    .children(&mut [
                        svg!("path", {
                            .attribute_signal("d", filter_list_modal.collapse_signal(index.get().unwrap_throw()).map(|visible| {
                                if visible {
                                    Some("M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z")
                                } else {
                                    Some("M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z")
                                }
                            }))
                        }),
                    ])
                })
            ])
        })
    }

    pub fn render<F>(filter_list_modal: Rc<Self>, f: F) -> Dom
    where
        F: Fn() + Clone + 'static,
    {
        html!("div", {
            .children(&mut [
                html!("div", {
                    .visible_signal(filter_list_modal.show.signal())
                    .class("reader-settings-background")
                    .event(clone!(filter_list_modal =>  move |_: events::Click| {
                        filter_list_modal.show.set(false);
                    }))
                }),
                html!("div", {
                    .class("reader-settings")
                    .style_important("padding-top", "0")
                    .class(["modal", "animate__animated", "animate__faster"])
                    .style("padding-bottom", "calc(env(safe-area-inset-bottom) + 0.5rem)")
                    .style("max-height", "80%")
                    .style("overflow", "auto")
                    .class_signal("animate__slideInUp", filter_list_modal.show.signal())
                    .class_signal("animate__slideOutDown", filter_list_modal.show.signal().map(|x| !x))
                    .visible_signal(filter_list_modal.first_render.signal().map(|x| !x))
                    .children(&mut[
                        html!("div", {
                            .style("display", "flex")
                            .style("position", "sticky")
                            .style("top", "0")
                            .style("padding", "0.5rem")
                            .style("justify-content", "space-between")
                            .style("margin-bottom", "0.5rem")
                            .style("background-color", "var(--modal--background-color)")
                            .children(&mut [
                                html!("span", {
                                    .style("font-size", "large")
                                    .text("Filters")
                                }),
                                html!("div", {
                                    .children(&mut [
                                        html!("button", {
                                            .text("Apply")
                                            .event(clone!(filter_list_modal => move |_: events::Click| {
                                                f();
                                                filter_list_modal.show.set(false);
                                            }))
                                        })
                                    ])
                                })
                            ])
                        })
                    ])
                    .children_signal_vec(filter_list_modal.filter_list.signal_vec_cloned().enumerate().map(clone!(filter_list_modal => move |(index, input)| match input {
                        Input::Text { name, state } => html!("div", {
                            .style("margin-bottom", "0.5rem")
                            .children(&mut [
                                html!("label", {
                                    .text(&name)
                                }),
                                html!("div", {
                                    .class("reader-settings-row")
                                    .style("display", "flex")
                                    .children(&mut [
                                        html!("input" => HtmlInputElement, {
                                            .attr("type", "text")
                                            .with_node!(input => {
                                                .event(clone!(state => move |_: events::Input| {
                                                    state.set(Some(input.value()));
                                                }))
                                            })
                                        })
                                    ])
                                })
                            ])
                        }),
                        Input::Checkbox { name, state } => html!("div", {
                            .style("margin-bottom", "0.5rem")
                            .style("display", "flex")
                            .style("justify-content", "space-between")
                            .children(&mut [
                                html!("label", {
                                    .text(&name)
                                }),
                                html!("div", {
                                    .class("reader-settings-row")
                                    .children(&mut [
                                        html!("input", {
                                            .attr("type", "checkbox")
                                            .attr_signal("checked", state.signal().map(|state|  state.and_then(|state| state.then(|| "checked"))))
                                        }),
                                    ])
                                })
                            ])
                        }),
                        Input::Select {
                            name,
                            values,
                            state,
                        } => html!("div", {
                            .style("margin-bottom", "0.5rem")
                            .style("display", "flex")
                            .style("justify-content", "space-between")
                            .children(&mut [
                                html!("label", {
                                    .text(&name)
                                }),
                                html!("div", {
                                    .class("reader-settings-row")
                                    .children(&mut [
                                        html!("select" => HtmlSelectElement, {
                                            .children_signal_vec(signal_vec::always(values).map(|value| html!("option", {
                                                .text(&value.to_string())
                                            })))
                                            .with_node!(select => {
                                                .event(clone!(select, state => move |_: events::Change| {
                                                    let selected_index = select.selected_index() as i64;
                                                    state.set((selected_index > 0).then(|| selected_index - 1));
                                                }))
                                            })
                                        }),
                                    ])
                                })
                            ])
                        }),
                        Input::Group { name, state } =>  html!("div", {
                            .style("margin-bottom", "0.5rem")
                            .children(&mut [
                                Self::render_collapsible_header(filter_list_modal.clone(), index.clone(), name.clone()),
                                html!("div" => HtmlElement, {
                                    .class("reader-settings-row")
                                    .visible_signal(filter_list_modal.collapse_signal(index.get().unwrap_throw()))
                                    .children_signal_vec(state.signal_vec_cloned().map(|value| match value {
                                        InputType::State { name, selected } => html!("div" => HtmlElement, {
                                            .style("margin", "0.5rem")
                                            .style("display", "flex")
                                            .style("align-items", "center")
                                            .with_node!(row => {
                                                .event(clone!(row, selected => move |_:events::Click| {
                                                    info!("clicked");
                                                    let input = row
                                                        .get_elements_by_tag_name("input")
                                                        .get_with_index(0)
                                                        .unwrap_throw()
                                                        .dyn_into::<HtmlInputElement>()
                                                        .unwrap_throw();
                                                    match selected.get() {
                                                        Some(true) => {
                                                            selected.set(Some(false));
                                                            input.set_checked(false);
                                                            input.set_indeterminate(false);
                                                            info!("false");
                                                        }
                                                        Some(false) => {
                                                            selected.set(None);
                                                            input.set_indeterminate(true);
                                                            info!("none");
                                                        }
                                                        None => {
                                                            selected.set(Some(true));
                                                            input.set_indeterminate(false);
                                                            input.set_checked(true);
                                                            info!("true");
                                                        }
                                                    }
                                                }))
                                            })
                                            .children(&mut [
                                                html!("input" => HtmlInputElement, {
                                                    .class("tri-state")
                                                    .style("margin-right", "0.5rem")
                                                    .attribute("type", "checkbox")
                                                    .after_inserted(clone!(selected => move |input| {
                                                        if selected.get().is_none() {
                                                            input.set_indeterminate(true);
                                                        }
                                                    }))
                                                    .attribute_signal("checked", selected.signal().map(|selected| if selected.unwrap_or_default() { Some("true") } else { None }))
                                                }),
                                                html!("label", {
                                                    .style_signal("text-decoration", selected.signal().map(|selected| if selected.map(|s| s == false).unwrap_or_default() { Some("line-through") } else { None }))
                                                    .text(&name)
                                                })
                                            ])
                                        }),
                                        _ => html!("div", {})
                                    }))
                                })
                            ])
                        }),
                        Input::Sort {
                            name,
                            values,
                            selection,
                        } =>  html!("div", {
                            .style("margin-bottom", "0.5rem")
                            .children(&mut [
                                Self::render_collapsible_header(filter_list_modal.clone(), index.clone(), name.clone()),
                                html!("div" => HtmlElement, {
                                    .class("reader-settings-row")
                                    .visible_signal(filter_list_modal.collapse_signal(index.get().unwrap_throw()))
                                    .children_signal_vec(signal_vec::always(values).map(clone!(index, name, selection => move |value| html!("div", {
                                        .style("margin", "0.5rem")
                                        .style("display", "flex")
                                        .style("align-items", "center")
                                        .children(&mut [
                                            html!("label", {
                                                .style("margin-right", "0.5rem")
                                                .children(&mut [
                                                    html!("input" => HtmlInputElement, {
                                                        .class("tri-state")
                                                        .attr("type", "radio")
                                                        .attr("name", &name)
                                                        .attr("id", &value.to_string())
                                                        .with_node!(input => {
                                                            .event(clone!(input, index, selection => move |_: events::Change| {
                                                                if input.checked() {
                                                                    selection.set(Some((index.get().unwrap_throw() as i64, true)));
                                                                }
                                                            }))
                                                        })
                                                    }),
                                                    svg!("svg", {
                                                        .attribute("xmlns", "http://www.w3.org/2000/svg")
                                                        .attribute("fill", "currentColor")
                                                        .attribute("viewBox", "0 0 20 20")
                                                        .class("icon")
                                                        .children(&mut [
                                                            svg!("path", {
                                                                .attribute_signal("d", selection.signal_cloned().map(clone!(index => move |selection| {
                                                                    if let Some((i, asc)) = selection {
                                                                        if index.get().unwrap_throw() == (i as usize) {
                                                                            if asc {
                                                                                Some("M5.293 9.707a1 1 0 010-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 01-1.414 1.414L11 7.414V15a1 1 0 11-2 0V7.414L6.707 9.707a1 1 0 01-1.414 0z")
                                                                            } else {
                                                                                Some("M14.707 10.293a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 111.414-1.414L9 12.586V5a1 1 0 012 0v7.586l2.293-2.293a1 1 0 011.414 0z")
                                                                            }
                                                                        } else {
                                                                            None
                                                                        }
                                                                    } else {
                                                                        None
                                                                    }
                                                                })))
                                                            }),
                                                        ])
                                                    })
                                                ])
                                            }),
                                            html!("label", {
                                                .attr("for", &value.to_string())
                                                .text(&value.to_string())
                                                .event(clone!(index, selection => move |_:events::Click| {
                                                    match selection.get()  {
                                                        Some((i, true)) if index.get().unwrap_throw() as i64 == i => {
                                                            selection.set(Some((i, false)));
                                                        }
                                                        Some((i, false)) if index.get().unwrap_throw() as i64 == i => {
                                                            selection.set(Some((i, true)));
                                                        }
                                                        _ => {}
                                                    }
                                                }))
                                            })
                                        ])
                                    }))))
                                })
                            ])
                        }),
                    })))
                })
            ])
        })
    }
}

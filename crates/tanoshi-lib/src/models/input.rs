use rquickjs::{FromJs, IntoJs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "js", derive(FromJs, IntoJs))]
#[cfg_attr(feature = "js", quickjs(untagged))]
pub enum InputType {
    String(String),
    Number(f64),
    Boolean(bool),
    State {
        name: String,
        selected: Option<bool>,
    },
}

impl From<String> for InputType {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for InputType {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<f64> for InputType {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for InputType {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "js", derive(FromJs, IntoJs))]
#[cfg_attr(feature = "js", quickjs(tag = "type"))]
pub enum Input {
    Text {
        name: String,
        state: Option<String>,
    },
    Checkbox {
        name: String,
        state: Option<bool>,
    },
    Select {
        name: String,
        values: Vec<InputType>,
        state: Option<i64>,
    },
    Group {
        name: String,
        state: Option<Vec<InputType>>,
    },
    Sort {
        name: String,
        values: Vec<InputType>,
        selection: Option<(i64, bool)>,
    },
}

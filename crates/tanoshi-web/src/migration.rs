use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;

use crate::utils::local_storage;

pub const MIGRATION_KEY: &str = "migration:state";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationState {
    // What we are migrating FROM (usually in your library)
    pub from_manga_id: i64,
    pub from_source_id: i64,
    pub from_title: String,

    // Where we are migrating TO
    pub to_source_id: i64,
}

pub fn set(state: &MigrationState) {
    let json = serde_json::to_string(state).unwrap_throw();
    local_storage().set(MIGRATION_KEY, &json).unwrap_throw();
}

pub fn get() -> Option<MigrationState> {
    local_storage()
        .get(MIGRATION_KEY)
        .unwrap_throw()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn clear() {
    let _ = local_storage().delete(MIGRATION_KEY);
}

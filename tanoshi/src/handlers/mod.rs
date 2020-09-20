pub mod auth;
pub mod favorites;
pub mod history;
pub mod manga;
pub mod updates;
pub mod static_files;

#[derive(Debug)]
pub struct TransactionReject {
    pub message: String,
}

impl warp::reject::Reject for TransactionReject{}
pub mod history {
    use crate::auth::Claims;
    use crate::history::{history::History, HistoryChapter};
    use sled::Db;
    use std::convert::Infallible;

    pub async fn get_history(
        source: String,
        title: String,
        claim: Claims,
        history: History,
        db: Db,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = history.get_history(claim.sub, source, title, db);
        Ok(warp::reply::json(&res))
    }

    pub async fn add_history(
        source: String,
        title: String,
        claim: Claims,
        chapter: HistoryChapter,
        history: History,
        db: Db,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = history.add_history(claim.sub, source, title, chapter, db);
        Ok(warp::reply::json(&res))
    }
}

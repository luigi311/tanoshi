pub mod history {
    use crate::auth::Claims;
    use crate::filters::favorites::favorites::favorites;
    use crate::history::{history::History, HistoryRequest};
    use sled::Tree;
    use std::collections::BTreeMap;
    use std::convert::Infallible;

    pub async fn get_history(
        source: String,
        title: String,
        claim: Claims,
        history: History,
        library_tree: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = history.get_history(claim.sub, source, title, library_tree);
        Ok(warp::reply::json(&res))
    }

    pub async fn add_history(
        claim: Claims,
        request: HistoryRequest,
        history: History,
        library_tree: Tree,
        scraper_tree: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = history.add_history(claim.sub, request, library_tree, scraper_tree);
        Ok(warp::reply::json(&res))
    }
}

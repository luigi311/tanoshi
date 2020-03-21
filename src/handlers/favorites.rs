pub mod favorites {
    use crate::auth::Claims;
    use crate::favorites::{favorites::Favorites, FavoriteManga};
    use sled::Tree;
    use std::convert::Infallible;

    pub async fn get_favorites(
        claim: Claims,
        fav: Favorites,
        db: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.get_favorites(claim.sub, db);
        Ok(warp::reply::json(&res))
    }

    pub async fn add_favorites(
        claim: Claims,
        manga: FavoriteManga,
        fav: Favorites,
        db: Tree,
        scraper_tree: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.add_favorite(claim.sub, manga, db);
        Ok(warp::reply::json(&res))
    }

    pub async fn remove_favorites(
        claim: Claims,
        manga: FavoriteManga,
        fav: Favorites,
        db: Tree,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.remove_favorites(claim.sub, manga, db);
        Ok(warp::reply::json(&res))
    }
}

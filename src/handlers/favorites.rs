pub mod favorites {
    use crate::auth::Claims;
    use crate::favorites::{favorites::Favorites, FavoriteManga};
    use std::convert::Infallible;

    pub async fn get_favorites(
        claim: Claims,
        fav: Favorites,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.get_favorites(claim.sub);
        Ok(warp::reply::json(&res))
    }

    pub async fn add_favorites(
        claim: Claims,
        manga: FavoriteManga,
        fav: Favorites,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.add_favorite(claim.sub, manga);
        Ok(warp::reply::json(&res))
    }

    pub async fn remove_favorites(
        claim: Claims,
        manga: FavoriteManga,
        fav: Favorites,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.remove_favorites(claim.sub, manga);
        Ok(warp::reply::json(&res))
    }
}

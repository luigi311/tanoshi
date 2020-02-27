pub mod favorites {
    use crate::favorites::{favorites::Favorites, FavoriteManga};
    use std::convert::Infallible;

    pub async fn get_favorites(
        username: String,
        fav: Favorites,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.get_favorites(username);
        Ok(warp::reply::json(&res))
    }

    pub async fn add_favorites(
        username: String,
        manga: FavoriteManga,
        fav: Favorites,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = fav.add_favorite(username, manga);
        Ok(warp::reply::json(&res))
    }
}

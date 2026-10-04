use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::Duration,
};

use async_graphql::{
    EmptyMutation, EmptySubscription, Request, Schema, SimpleObject,
    dataloader::{DataLoader, Loader},
};
use tanoshi::{
    domain::repositories::library::LibraryRepository,
    infrastructure::{
        auth::Claims,
        database::{Pool, establish_connection},
        domain::repositories::{
            download::DownloadRepositoryImpl, history::HistoryRepositoryImpl,
            library::LibraryRepositoryImpl, manga::MangaRepositoryImpl,
            tracker::TrackerRepositoryImpl,
        },
    },
    presentation::graphql::{
        loader::{UserFavoriteId, UserFavoritePath},
        manga::Manga,
        schema::DatabaseLoader,
    },
};

struct Fixture {
    dir: PathBuf,
    pool: Pool,
}

impl Fixture {
    async fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tanoshi-favorite-membership-{:016x}",
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        let pool = establish_connection(dir.join("test.db").to_str().unwrap(), true)
            .await
            .unwrap();
        // Author and genre are deliberately NULL: membership lookups must not
        // materialize manga metadata, including metadata for unrelated favorites.
        sqlx::raw_sql(
            r#"
            INSERT INTO user (id, username, password) VALUES
                (1, 'reader-1', 'unused'), (2, 'reader-2', 'unused');
            INSERT INTO manga (id, source_id, title, path, cover_url, date_added) VALUES
                (1, 1, 'Source 1 favorite', '/same-path', '', CURRENT_TIMESTAMP),
                (2, 2, 'Source 2 favorite', '/same-path', '', CURRENT_TIMESTAMP),
                (3, 1, 'Not a favorite', '/not-favorite', '', CURRENT_TIMESTAMP),
                (4, 3, 'Unrequested favorite', '/unrequested', '', CURRENT_TIMESTAMP);
            INSERT INTO user_library (user_id, manga_id) VALUES (1, 1), (1, 4), (2, 2);
            "#,
        )
        .execute(pool.write())
        .await
        .unwrap();
        Self { dir, pool }
    }

    fn loader(&self) -> DatabaseLoader {
        DatabaseLoader::new(
            HistoryRepositoryImpl::new(self.pool.clone()),
            LibraryRepositoryImpl::new(self.pool.clone()),
            MangaRepositoryImpl::new(self.pool.clone()),
            TrackerRepositoryImpl::new(self.pool.clone(), None, None),
            DownloadRepositoryImpl::new(self.pool.clone()),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn favorite_membership_queries_return_only_requested_matches() {
    let fixture = Fixture::new().await;
    let repo = LibraryRepositoryImpl::new(fixture.pool.clone());
    let ids = [1, 2, 3, 1, 999];
    assert_eq!(
        repo.get_favorite_manga_ids(1, &ids).await.unwrap(),
        HashSet::from([1])
    );
    assert_eq!(
        repo.get_favorite_manga_ids(2, &ids).await.unwrap(),
        HashSet::from([2])
    );
    let paths = vec![
        "/same-path".into(),
        "/not-favorite".into(),
        "/missing".into(),
        "/same-path".into(),
    ];
    for user_id in [1, 2] {
        for source_id in [1, 2, 3] {
            let expected = if user_id == source_id {
                HashSet::from(["/same-path".to_owned()])
            } else {
                HashSet::new()
            };
            assert_eq!(
                repo.get_favorite_manga_paths(user_id, source_id, &paths)
                    .await
                    .unwrap(),
                expected,
                "user {user_id}, source {source_id}"
            );
        }
    }
    fixture.pool.close().await;
}

#[tokio::test]
async fn favorite_membership_queries_handle_empty_lists() {
    let fixture = Fixture::new().await;
    let repo = LibraryRepositoryImpl::new(fixture.pool.clone());
    assert!(
        repo.get_favorite_manga_ids(1, &[])
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.get_favorite_manga_paths(1, 1, &[])
            .await
            .unwrap()
            .is_empty()
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn favorite_loaders_return_only_requested_user_and_source_matches() {
    let fixture = Fixture::new().await;
    let loader = fixture.loader();
    let ids = loader
        .load(&[
            UserFavoriteId(1, 1),
            UserFavoriteId(1, 2),
            UserFavoriteId(2, 1),
            UserFavoriteId(2, 2),
            UserFavoriteId(1, 999),
        ])
        .await
        .unwrap();
    assert_eq!(
        ids,
        HashMap::from([(UserFavoriteId(1, 1), true), (UserFavoriteId(2, 2), true)])
    );
    let paths = loader
        .load(&[
            UserFavoritePath(1, 1, "/same-path".into()),
            UserFavoritePath(1, 2, "/same-path".into()),
            UserFavoritePath(2, 1, "/same-path".into()),
            UserFavoritePath(2, 2, "/same-path".into()),
            UserFavoritePath(1, 3, "/same-path".into()),
            UserFavoritePath(1, 1, "/not-favorite".into()),
            UserFavoritePath(1, 1, "/same-path".into()),
        ])
        .await
        .unwrap();
    assert_eq!(
        paths,
        HashMap::from([
            (UserFavoritePath(1, 1, "/same-path".into()), true),
            (UserFavoritePath(2, 2, "/same-path".into()), true),
        ])
    );
    fixture.pool.close().await;
}

#[derive(SimpleObject)]
struct FavoriteQuery {
    catalogue: Vec<Manga>,
}

fn request(user_id: i64) -> Request {
    Request::new("{ catalogue { id isFavorite } }").data(Claims {
        sub: user_id,
        username: format!("reader-{user_id}"),
        is_admin: false,
        exp: usize::MAX,
    })
}

#[tokio::test]
async fn catalogue_favorites_keep_sources_and_concurrent_users_separate() {
    let fixture = Fixture::new().await;
    let mut catalogue: Vec<_> = [1, 2, 3]
        .into_iter()
        .map(|source_id| Manga {
            id: 0,
            source_id,
            path: "/same-path".into(),
            ..Manga::default()
        })
        .collect();
    catalogue.push(Manga {
        id: 2,
        source_id: 2,
        path: "/same-path".into(),
        ..Manga::default()
    });
    // Like production, both requests share one schema-owned DataLoader.
    let schema = Schema::build(
        FavoriteQuery { catalogue },
        EmptyMutation,
        EmptySubscription,
    )
    .data(DataLoader::new(fixture.loader(), tokio::spawn).delay(Duration::from_millis(10)))
    .finish();
    let (user_1, user_2) = tokio::join!(schema.execute(request(1)), schema.execute(request(2)),);
    for (response, expected) in [
        (user_1, vec![true, false, false, false]),
        (user_2, vec![false, true, false, true]),
    ] {
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let favorites: Vec<_> = data["catalogue"]
            .as_array()
            .unwrap()
            .iter()
            .map(|manga| manga["isFavorite"].as_bool().unwrap())
            .collect();
        assert_eq!(favorites, expected);
    }
    fixture.pool.close().await;
}

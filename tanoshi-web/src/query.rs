use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use wasm_bindgen::prelude::*;
use web_sys::window;

type NaiveDateTime = String;

use crate::common::Cover;

fn graphql_url() -> String {
    [
        window()
            .unwrap()
            .document()
            .unwrap()
            .location()
            .unwrap()
            .origin()
            .unwrap(),
        "/graphql".to_string(),
    ]
    .join("")
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/browse_source.graphql",
    response_derives = "Debug, Clone"
)]
pub struct BrowseSource;

pub async fn fetch_manga_from_source(
    source_id: i64,
    page: i64,
    keyword: Option<String>,
    sort_by: browse_source::SortByParam,
    sort_order: browse_source::SortOrderParam,
) -> Result<Vec<Cover>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = browse_source::Variables {
        source_id: Some(source_id),
        keyword,
        page: Some(page),
        sort_by: Some(sort_by),
        sort_order: Some(sort_order),
    };
    let request_body = BrowseSource::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<browse_source::ResponseData> = res.json().await?;
    let list = response_body.data.ok_or("no data")?.browse_source;

    let covers = list
        .iter()
        .map(|item| Cover::new(item.id, item.title.clone(), item.cover_url.clone()))
        .collect();
    Ok(covers)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/browse_favorites.graphql",
    response_derives = "Debug"
)]
pub struct BrowseFavorites;

pub async fn fetch_manga_from_favorite() -> Result<Vec<Cover>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = browse_favorites::Variables {};
    let request_body = BrowseFavorites::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<browse_favorites::ResponseData> = res.json().await?;
    let list = response_body.data.ok_or("no data")?.library;

    Ok(list
        .iter()
        .map(|item| Cover::new(item.id, item.title.clone(), item.cover_url.clone()))
        .collect())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_manga_detail.graphql",
    response_derives = "Debug"
)]
pub struct FetchMangaDetail;

pub async fn fetch_manga_detail(
    id: i64,
) -> Result<fetch_manga_detail::FetchMangaDetailManga, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_manga_detail::Variables { id: Some(id) };
    let request_body = FetchMangaDetail::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<fetch_manga_detail::ResponseData> = res.json().await?;
    let manga = response_body.data.ok_or("no data")?.manga.unwrap_throw();

    Ok(manga)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_chapter.graphql",
    response_derives = "Debug"
)]
pub struct FetchChapter;

pub async fn fetch_chapter(
    chapter_id: i64,
) -> Result<fetch_chapter::FetchChapterChapter, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_chapter::Variables {
        chapter_id: Some(chapter_id),
    };
    let request_body = FetchChapter::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<fetch_chapter::ResponseData> = res.json().await?;
    let manga = response_body.data.ok_or("no data")?.chapter.unwrap_throw();

    Ok(manga)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/add_to_library.graphql",
    response_derives = "Debug"
)]
pub struct AddToLibrary;

pub async fn add_to_library(manga_id: i64) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = add_to_library::Variables {
        manga_id: Some(manga_id),
    };
    let request_body = AddToLibrary::build_query(var);
    let _ = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/delete_from_library.graphql",
    response_derives = "Debug"
)]
pub struct DeleteFromLibrary;

pub async fn delete_from_library(manga_id: i64) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = delete_from_library::Variables {
        manga_id: Some(manga_id),
    };
    let request_body = DeleteFromLibrary::build_query(var);
    let _ = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_page_read_at.graphql",
    response_derives = "Debug"
)]
pub struct UpdatePageReadAt;

pub async fn update_page_read_at(page_id: i64) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = update_page_read_at::Variables {
        page_id: Some(page_id),
    };
    let request_body = UpdatePageReadAt::build_query(var);
    let _ = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_recent_updates.graphql",
    response_derives = "Debug"
)]
pub struct FetchRecentUpdates;

pub async fn fetch_recent_updates(cursor: Option<String>) -> Result<fetch_recent_updates::FetchRecentUpdatesRecentUpdates, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_recent_updates::Variables {
        first: Some(20),
        cursor: cursor,
    };
    let request_body = FetchRecentUpdates::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<fetch_recent_updates::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.recent_updates)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_histories.graphql",
    response_derives = "Debug"
)]
pub struct FetchHistories;

pub async fn fetch_histories(cursor: Option<String>) -> Result<fetch_histories::FetchHistoriesRecentChapters, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_histories::Variables {
        first: Some(20),
        cursor: cursor,
    };
    let request_body = FetchHistories::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<fetch_histories::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.recent_chapters)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_sources.graphql",
    response_derives = "Debug"
)]
pub struct FetchSources;

pub async fn fetch_sources() -> Result<std::vec::Vec<fetch_sources::FetchSourcesInstalledSources>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_sources::Variables {};
    let request_body = FetchSources::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<fetch_sources::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.installed_sources)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_source.graphql",
    response_derives = "Debug"
)]
pub struct FetchSourceDetail;

pub async fn fetch_source(source_id: i64) -> Result<Option<fetch_source_detail::FetchSourceDetailSource>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_source_detail::Variables {
        source_id: Some(source_id),
    };
    let request_body = FetchSourceDetail::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<fetch_source_detail::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/login.graphql",
    response_derives = "Debug"
)]
pub struct UserLogin;

pub async fn user_login(username: String, password: String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = user_login::Variables {
        username: Some(username),
        password: Some(password),
    };
    let request_body = UserLogin::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<user_login::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.login)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/register.graphql",
    response_derives = "Debug"
)]
pub struct UserRegister;

pub async fn user_register(username: String, password: String) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = user_register::Variables {
        username: Some(username),
        password: Some(password),
    };
    let request_body = UserRegister::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<user_register::ResponseData> = res.json().await?;
    let user_id = response_body.data.ok_or("no data")?.register;
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_server_status.graphql",
    response_derives = "Debug"
)]
pub struct FetchServerStatus;

pub async fn server_status() -> Result<fetch_server_status::FetchServerStatusServerStatus, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_server_status::Variables {};
    let request_body = FetchServerStatus::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;
    
    let response_body: Response<fetch_server_status::ResponseData> = res.json().await?;
    Ok(response_body.data.ok_or("no data")?.server_status)
}
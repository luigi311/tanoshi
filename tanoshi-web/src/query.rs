use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use wasm_bindgen::prelude::*;
use web_sys::window;

type NaiveDateTime = String;

use crate::{common::Cover, utils::local_storage};

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
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
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
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<browse_source::ResponseData> = res.json().await?;
    let list = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.browse_source,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

    let covers = list
        .iter()
        .map(|item| {
            Cover::new(
                item.id,
                item.source_id,
                item.path.clone(),
                item.title.clone(),
                item.cover_url.clone(),
                item.is_favorite,
            )
        })
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

pub async fn fetch_manga_from_favorite(refresh: bool) -> Result<Vec<Cover>, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = browse_favorites::Variables {
        refresh: Some(refresh),
    };
    let request_body = BrowseFavorites::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<browse_favorites::ResponseData> = res.json().await?;

    let list = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.library,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

    Ok(list
        .iter()
        .map(|item| {
            Cover::new(
                item.id,
                item.source_id,
                item.path.clone(),
                item.title.clone(),
                item.cover_url.clone(),
                false,
            )
        })
        .collect())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_manga_by_source_path.graphql",
    response_derives = "Debug"
)]
pub struct FetchMangaBySourcePath;

pub async fn fetch_manga_by_source_path(
    source_id: i64,
    path: String,
) -> Result<fetch_manga_by_source_path::FetchMangaBySourcePathMangaBySourcePath, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_manga_by_source_path::Variables {
        source_id: Some(source_id),
        path: Some(path),
    };
    let request_body = FetchMangaBySourcePath::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<fetch_manga_by_source_path::ResponseData> = res.json().await?;
    let manga = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.manga_by_source_path,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

    Ok(manga)
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
    refresh: bool,
) -> Result<fetch_manga_detail::FetchMangaDetailManga, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_manga_detail::Variables {
        id: Some(id),
        refresh: Some(refresh),
    };
    let request_body = FetchMangaDetail::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<fetch_manga_detail::ResponseData> = res.json().await?;
    let manga = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.manga,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

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
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_chapter::Variables {
        chapter_id: Some(chapter_id),
    };
    let request_body = FetchChapter::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<fetch_chapter::ResponseData> = res.json().await?;
    let chapter = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.chapter,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

    Ok(chapter)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/add_to_library.graphql",
    response_derives = "Debug"
)]
pub struct AddToLibrary;

pub async fn add_to_library(manga_id: i64) -> Result<(), Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = add_to_library::Variables {
        manga_id: Some(manga_id),
    };
    let request_body = AddToLibrary::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<add_to_library::ResponseData> = res.json().await?;
    let _ = match (response_body.data, response_body.errors) {
        (Some(_), _) => {}
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

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
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = delete_from_library::Variables {
        manga_id: Some(manga_id),
    };
    let request_body = DeleteFromLibrary::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<delete_from_library::ResponseData> = res.json().await?;
    let _ = match (response_body.data, response_body.errors) {
        (Some(_), _) => {}
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };

    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_page_read_at.graphql",
    response_derives = "Debug"
)]
pub struct UpdatePageReadAt;

pub async fn update_page_read_at(chapter_id: i64, page: i64) -> Result<(), Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = update_page_read_at::Variables {
        chapter_id: Some(chapter_id),
        page: Some(page),
    };
    let request_body = UpdatePageReadAt::build_query(var);
    let _ = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
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

pub async fn fetch_recent_updates(
    cursor: Option<String>,
) -> Result<fetch_recent_updates::FetchRecentUpdatesRecentUpdates, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_recent_updates::Variables {
        first: Some(20),
        cursor,
    };
    let request_body = FetchRecentUpdates::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_recent_updates::ResponseData> = res.json().await?;
    let recent_updates = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.recent_updates,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(recent_updates)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_histories.graphql",
    response_derives = "Debug"
)]
pub struct FetchHistories;

pub async fn fetch_histories(
    cursor: Option<String>,
) -> Result<fetch_histories::FetchHistoriesRecentChapters, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_histories::Variables {
        first: Some(20),
        cursor,
    };
    let request_body = FetchHistories::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_histories::ResponseData> = res.json().await?;
    let recent_chapters = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.recent_chapters,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(recent_chapters)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_sources.graphql",
    response_derives = "Debug"
)]
pub struct FetchSources;

pub async fn fetch_sources(
) -> Result<std::vec::Vec<fetch_sources::FetchSourcesInstalledSources>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_sources::Variables {};
    let request_body = FetchSources::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_sources::ResponseData> = res.json().await?;
    let installed_sources = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.installed_sources,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(installed_sources)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_all_sources.graphql",
    response_derives = "Debug"
)]
pub struct FetchAllSources;

pub async fn fetch_all_sources() -> Result<fetch_all_sources::ResponseData, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_all_sources::Variables {};
    let request_body = FetchAllSources::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_all_sources::ResponseData> = res.json().await?;
    let data = match (response_body.data, response_body.errors) {
        (Some(data), _) => data,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(data)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_source.graphql",
    response_derives = "Debug"
)]
pub struct FetchSourceDetail;

#[allow(dead_code)]
pub async fn fetch_source(
    source_id: i64,
) -> Result<fetch_source_detail::FetchSourceDetailSource, Box<dyn Error>> {
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
    let source = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.source,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/install_source.graphql",
    response_derives = "Debug"
)]
pub struct InstallSource;

pub async fn install_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = install_source::Variables {
        source_id: Some(source_id),
    };
    let request_body = InstallSource::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<install_source::ResponseData> = res.json().await?;
    let install_source = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.install_source,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(install_source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_source.graphql",
    response_derives = "Debug"
)]
pub struct UpdateSource;

pub async fn update_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = update_source::Variables {
        source_id: Some(source_id),
    };
    let request_body = UpdateSource::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<update_source::ResponseData> = res.json().await?;
    let update_source = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.update_source,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(update_source)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/uninstall_source.graphql",
    response_derives = "Debug"
)]
pub struct UninstallSource;

pub async fn uninstall_source(source_id: i64) -> Result<i64, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = uninstall_source::Variables {
        source_id: Some(source_id),
    };
    let request_body = UninstallSource::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<uninstall_source::ResponseData> = res.json().await?;
    let uninstall_source = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.uninstall_source,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(uninstall_source)
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
    let login = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.login,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(login)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_users.graphql",
    response_derives = "Debug"
)]
pub struct FetchUserList;

pub async fn fetch_users() -> Result<
    (
        fetch_user_list::FetchUserListMe,
        Vec<fetch_user_list::FetchUserListUsers>,
    ),
    Box<dyn Error>,
> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_user_list::Variables {};
    let request_body = FetchUserList::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_user_list::ResponseData> = res.json().await?;
    let (me, users) = match (response_body.data, response_body.errors) {
        (Some(data), _) => (data.me, data.users),
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok((me, users))
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_me.graphql",
    response_derives = "Debug"
)]
pub struct FetchMe;

pub async fn fetch_me() -> Result<fetch_me::FetchMeMe, Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = fetch_me::Variables {};
    let request_body = FetchMe::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_me::ResponseData> = res.json().await?;
    let me = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.me,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(me)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/register.graphql",
    response_derives = "Debug"
)]
pub struct UserRegister;

pub async fn user_register(
    username: String,
    password: String,
    is_admin: bool,
) -> Result<(), Box<dyn Error>> {
    let token = local_storage().get("token").unwrap_throw();
    let client = reqwest::Client::new();
    let var = user_register::Variables {
        username: Some(username),
        password: Some(password),
        is_admin: Some(is_admin),
    };
    let request_body = UserRegister::build_query(var);
    let mut req = client.post(&graphql_url());
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let res = req.json(&request_body).send().await?;

    let response_body: Response<user_register::ResponseData> = res.json().await?;
    let _ = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.register,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/change_password.graphql",
    response_derives = "Debug"
)]
pub struct ChangeUserPassword;

pub async fn change_password(
    old_password: String,
    new_password: String,
) -> Result<(), Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = change_user_password::Variables {
        old_password: Some(old_password),
        new_password: Some(new_password),
    };
    let request_body = ChangeUserPassword::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<change_user_password::ResponseData> = res.json().await?;
    match (response_body.data, response_body.errors) {
        (Some(_), _) => Ok(()),
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => Err("no data".into()),
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/update_profile.graphql",
    response_derives = "Debug"
)]
pub struct UpdateProfile;

pub async fn update_profile(telegram_chat_id: Option<i64>) -> Result<(), Box<dyn Error>> {
    let token = local_storage()
        .get("token")
        .unwrap_throw()
        .ok_or("no token")?;
    let client = reqwest::Client::new();
    let var = update_profile::Variables { telegram_chat_id };
    let request_body = UpdateProfile::build_query(var);
    let res = client
        .post(&graphql_url())
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<update_profile::ResponseData> = res.json().await?;
    match (response_body.data, response_body.errors) {
        (Some(_), _) => Ok(()),
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => Err("no data".into()),
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/fetch_server_status.graphql",
    response_derives = "Debug"
)]
pub struct FetchServerStatus;

pub async fn server_status(
) -> Result<fetch_server_status::FetchServerStatusServerStatus, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let var = fetch_server_status::Variables {};
    let request_body = FetchServerStatus::build_query(var);
    let res = client
        .post(&graphql_url())
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<fetch_server_status::ResponseData> = res.json().await?;
    let server_status = match (response_body.data, response_body.errors) {
        (Some(data), _) => data.server_status,
        (_, Some(errors)) => {
            return Err(errors
                .iter()
                .map(|e| format!("{}", e))
                .collect::<Vec<String>>()
                .join(", ")
                .into());
        }
        _ => {
            return Err("no data".into());
        }
    };
    Ok(server_status)
}

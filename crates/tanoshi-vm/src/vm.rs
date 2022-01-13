use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, bail, Result};
use fnv::FnvHashMap;
use libloading::Library;
use tanoshi_lib::prelude::{ChapterInfo, Input, MangaInfo, PluginDeclaration, SourceInfo};
use tokio::sync::oneshot;

use crate::prelude::Source;

#[cfg(target_os = "windows")]
pub(crate) const PLUGIN_EXTENSION: &str = "dll";
#[cfg(target_os = "macos")]
pub(crate) const PLUGIN_EXTENSION: &str = "dylib";
#[cfg(target_os = "linux")]
pub(crate) const PLUGIN_EXTENSION: &str = "so";

pub enum SourceCommand {
    Insert(Source),
    Unload(i64),
    Exists(i64, oneshot::Sender<bool>),
    List(oneshot::Sender<Vec<SourceInfo>>),
    GetSourceInfo(i64, oneshot::Sender<Result<SourceInfo>>),
    GetFilterList(i64, oneshot::Sender<Result<Vec<Input>>>),
    GetPreferences(i64, oneshot::Sender<Result<Vec<Input>>>),
    SetPreferences(i64, Vec<Input>, oneshot::Sender<Result<()>>),
    GetPopularManga(i64, i64, oneshot::Sender<Result<Vec<MangaInfo>>>),
    GetLatestManga(i64, i64, oneshot::Sender<Result<Vec<MangaInfo>>>),
    SearchManga(
        i64,
        i64,
        Option<String>,
        Option<Vec<Input>>,
        oneshot::Sender<Result<Vec<MangaInfo>>>,
    ),
    GetMangaDetail(i64, String, oneshot::Sender<Result<MangaInfo>>),
    GetChapters(i64, String, oneshot::Sender<Result<Vec<ChapterInfo>>>),
    GetPages(i64, String, oneshot::Sender<Result<Vec<String>>>),
}

pub(crate) fn run<P: AsRef<Path>>(extension_dir: P, rx: flume::Receiver<SourceCommand>) {
    let extension_dir = PathBuf::new().join(extension_dir);
    std::thread::Builder::new()
        .name("extension_main_thread".to_string())
        .spawn(move || {
            if let Err(e) = extension_main_thread(extension_dir, rx) {
                error!("extension_main_thread exit: {}", e);
            }
        })
        .expect("failed to spawn extension main thread");
}

fn extension_main_thread<P: AsRef<Path>>(
    extension_dir: P,
    rx: flume::Receiver<SourceCommand>,
) -> Result<()> {
    let extension_dir = PathBuf::new().join(extension_dir);
    let mut extensions = FnvHashMap::default();

    loop {
        match rx.recv()? {
            SourceCommand::Insert(source) => {
                let info = source.extension.get().map(|s| s.get_source_info()).unwrap();
                extensions.insert(info.id, source);
            }
            SourceCommand::Unload(source_id) => {
                if let Some(source) = extensions
                    .remove(&source_id)
                    .and_then(|s| s.extension.get().map(|s| s.get_source_info()))
                {
                    std::fs::remove_file(
                        extension_dir
                            .join(&source.name.to_lowercase())
                            .with_extension(PLUGIN_EXTENSION),
                    )?;
                }
            }
            SourceCommand::Exists(source_id, tx) => tx
                .send(extensions.contains_key(&source_id))
                .expect("channel closed"),
            SourceCommand::List(tx) => {
                let list = extensions
                    .values()
                    .filter_map(|s| s.extension.get().map(|s| s.get_source_info()))
                    .collect();
                tx.send(list).expect("channel closed");
            }
            SourceCommand::GetSourceInfo(source_id, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .map(|ext| ext.get_source_info());
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetFilterList(source_id, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .map(|ext| ext.filter_list());
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetPreferences(source_id, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_preferences());
                tx.send(res).expect("channel closed");
            }
            SourceCommand::SetPreferences(source_id, preferences, tx) => {
                let res = extensions
                    .get_mut(&source_id)
                    .and_then(|s| s.extension.get_mut())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.set_preferences(preferences));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetPopularManga(source_id, page, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_popular_manga(page));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetLatestManga(source_id, page, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_latest_manga(page));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::SearchManga(source_id, page, query, filters, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.search_manga(page, query, filters));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetMangaDetail(source_id, path, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_manga_detail(path));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetChapters(source_id, path, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_chapters(path));
                tx.send(res).expect("channel closed");
            }
            SourceCommand::GetPages(source_id, path, tx) => {
                let res = extensions
                    .get(&source_id)
                    .and_then(|s| s.extension.get())
                    .ok_or_else(|| anyhow!("source with id {} doesn't exist", source_id))
                    .and_then(|ext| ext.get_pages(path));
                tx.send(res).expect("channel closed");
            }
        }
    }
}

pub(crate) fn load<P: AsRef<Path>>(extension_dir: P, name: &str) -> Result<Source> {
    let library_path = PathBuf::new().join(extension_dir).join(name);
    info!("load {:?}", library_path.display());

    #[cfg(target_os = "macos")]
    if let Err(e) = std::process::Command::new("install_name_tool")
        .current_dir(library_path.parent().unwrap())
        .arg("-id")
        .arg("''")
        .arg(library_path.file_name().unwrap())
        .output()
    {
        error!("failed to run install_name_tool: {}", e);
    }

    unsafe {
        let library = Arc::new(Library::new(&library_path)?);

        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

        if decl.rustc_version != tanoshi_lib::RUSTC_VERSION {
            bail!(
                "Version mismatch: extension.rustc_version={} != tanoshi_lib.rustc_version={}",
                decl.rustc_version,
                tanoshi_lib::RUSTC_VERSION,
            );
        }

        if decl.core_version != tanoshi_lib::LIB_VERSION {
            bail!(
                "Version mismatch: extension.lib_version={} != tanoshi_lib::lib_version={}",
                tanoshi_lib::RUSTC_VERSION,
                tanoshi_lib::LIB_VERSION
            );
        }

        let mut registrar = Source::new(Arc::clone(&library));
        (decl.register)(&mut registrar);

        Ok(registrar)
    }
}

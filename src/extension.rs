use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tanoshi_lib::prelude::{
    Chapter, Extension, ExtensionResult, Manga, Param, Request, Response, Source,
};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot::Sender,
    },
    task::JoinHandle,
    time::timeout,
};
use wasmer::{
    imports, ChainableNamedResolver, Function, Instance, Module, Store, Universal, WasmerEnv,
};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use crate::{config::Config, local::{self, Local}};

#[derive(Debug)]
pub enum Command {
    Load(String),
    Unload(i64),
    Exist(i64, Sender<bool>),
    Detail(i64, Sender<Source>),
    GetMangaList(i64, Param, Sender<Vec<Manga>>),
    GetMangaInfo(i64, String, Sender<Manga>),
    GetChapters(i64, String, Sender<Vec<Chapter>>),
    GetPages(i64, String, Sender<Vec<String>>),
}

#[derive(Debug, Clone)]
pub struct ExtensionBus {
    extension_dir_path: String,
    tx: UnboundedSender<Command>,
}

impl ExtensionBus {
    pub fn new(extension_dir_path: String, tx: UnboundedSender<Command>) -> Self {
        Self {
            extension_dir_path,
            tx,
        }
    }

    pub async fn install(
        &self,
        name: String,
        contents: &Bytes,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(self.extension_dir_path.clone())
            .join(name)
            .with_extension("wasm");
        std::fs::write(path.clone(), contents)?;

        Ok(self.tx.send(Command::Load(
            path.to_str().ok_or("path can't to string")?.to_string(),
        ))?)
    }

    pub async fn unload(&self, source_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.tx.send(Command::Unload(source_id))?)
    }

    pub async fn exist(&self, source_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Exist(source_id, tx))?;

        let exist = timeout(Duration::from_secs(30), rx).await??;
        Ok(exist)
    }

    pub async fn detail(&self, source_id: i64) -> Result<Source, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Detail(source_id, tx))?;

        let source = timeout(Duration::from_secs(30), rx).await??;
        Ok(source)
    }

    pub async fn get_manga_list(
        &self,
        source_id: i64,
        param: Param,
    ) -> Result<Vec<Manga>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetMangaList(source_id, param, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_manga_info(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Manga, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetMangaInfo(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<Chapter>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetChapters(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }

    pub async fn get_pages(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::GetPages(source_id, path, tx))?;

        let manga = timeout(Duration::from_secs(30), rx).await??;
        Ok(manga)
    }
}

#[derive(WasmerEnv, Clone)]
struct ExtensionEnv {
    wasi_env: WasiEnv,
}

struct ExtensionProxy {
    instance: Instance,
    path: String,
    env: ExtensionEnv,
}

impl ExtensionProxy {
    pub fn load(store: &Store, path: String) -> Result<Box<dyn Extension>, Box<dyn std::error::Error>> {
        let input = Pipe::new();
        let output = Pipe::new();
        let mut wasi_env = WasiState::new("tanoshi")
            .stdin(Box::new(input))
            .stdout(Box::new(output))
            .finalize()?;

        let extension_path = PathBuf::from(path.clone());

        let wasm_bytes = std::fs::read(extension_path)?;
        let module = Module::new(&store, wasm_bytes)?;

        let import_object = wasi_env.import_object(&module)?;

        let env = ExtensionEnv { wasi_env };

        let tanoshi = imports! {
            "tanoshi" => {
                "host_http_request" => Function::new_native_with_env(&store, env.clone(), host_http_request)
            }
        };

        let instance = Instance::new(&module, &tanoshi.chain_back(import_object))?;

        Ok(Box::new(ExtensionProxy {
            instance,
            path,
            env,
        }))
    }

    fn call<T>(&self, name: &str) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let res = self.instance.exports.get_function(name)?;
        res.call(&[])?;
        let object_str = wasi_read(&self.env)?;
        Ok(ron::from_str(&object_str)?)
    }

    fn call_with_args<T>(
        &self,
        name: &str,
        param: &impl Serialize,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let res = self.instance.exports.get_function(name)?;
        if let Err(e) = wasi_write(&self.env, &param) {
            error!("error write to wasi: {}", e);
        }
        res.call(&[])?;
        let object_str = wasi_read(&self.env)?;
        Ok(ron::from_str(&object_str)?)
    }
}

impl Extension for ExtensionProxy {
    fn detail(&self) -> Source {
        self.call("detail").unwrap_or(Source {
            id: 0,
            name: "".to_string(),
            url: "".to_string(),
            version: "".to_string(),
            icon: "".to_string(),
            need_login: false,
        })
    }

    fn get_manga_list(&self, param: Param) -> ExtensionResult<Vec<Manga>> {
        match self.call_with_args("get_manga_list", &param) {
            Ok(res) => res,
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
    }

    fn get_manga_info(&self, path: String) -> ExtensionResult<Manga> {
        match self.call_with_args("get_manga_info", &path) {
            Ok(res) => res,
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
    }

    fn get_chapters(&self, path: String) -> ExtensionResult<Vec<Chapter>> {
        match self.call_with_args("get_chapters", &path) {
            Ok(res) => res,
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
    }

    fn get_pages(&self, path: String) -> ExtensionResult<Vec<String>> {
        match self.call_with_args("get_pages", &path) {
            Ok(res) => res,
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
    }
}

pub fn start(cfg: Config) -> (JoinHandle<()>, UnboundedSender<Command>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async {
        extension_thread(cfg, rx).await;
    });

    (handle, tx)
}

pub async fn load(
    extention_dir_path: String,
    tx: UnboundedSender<Command>,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::read_dir(extention_dir_path.clone()) {
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::create_dir_all(extention_dir_path.clone());
        }
    }

    for entry in std::fs::read_dir(extention_dir_path.clone())?
        .into_iter()
        .filter(move |path| {
            if let Ok(p) = path {
                let ext = p
                    .clone()
                    .path()
                    .extension()
                    .unwrap_or("".as_ref())
                    .to_owned();
                if ext == "wasm" {
                    return true;
                }
            }
            return false;
        })
    {
        let path = entry?.path();
        info!("load plugin from {:?}", path.clone());
        tx.send(Command::Load(
            path.to_str().ok_or("no path str")?.to_string(),
        ))?;
    }

    Ok(())
}

async fn extension_thread(cfg: Config, extension_receiver: UnboundedReceiver<Command>) {
    let mut recv = extension_receiver;
    let mut extension_map: HashMap<i64, Box<dyn Extension>> = HashMap::new();
    extension_map.insert(local::ID, Local::new(cfg.local_path));

    let compiler = Cranelift::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);

    loop {
        let cmd = recv.recv().await;
        if let Some(cmd) = cmd {
            match cmd {
                Command::Load(path) => match ExtensionProxy::load(&store, path) {
                    Ok(proxy) => {
                        let source = proxy.detail();
                        info!("loaded: {:?}", source);
                        extension_map.insert(source.id, proxy);
                    }
                    Err(e) => {
                        error!("error load extension: {}", e);
                    }
                },
                Command::Unload(source_id) => {
                    extension_map.remove(&source_id);
                }
                Command::Exist(source_id, tx) => {
                    let exist = extension_map.get(&source_id).is_some();
                    if let Err(_) = tx.send(exist) {
                        error!("receiver dropped");
                    }
                }
                Command::Detail(source_id, tx) => match extension_map.get(&source_id) {
                    Some(proxy) => {
                        let res = proxy.detail();
                        if let Err(_) = tx.send(res) {
                            error!("receiver dropped");
                        }
                    }
                    None => {
                        error!("extension with id {} not found", source_id);
                    }
                },
                Command::GetMangaList(source_id, param, tx) => {
                    process(&extension_map, source_id, tx, |proxy| {
                        proxy.get_manga_list(param.clone())
                    });
                }
                Command::GetMangaInfo(source_id, path, tx) => {
                    process(&extension_map, source_id, tx, |proxy| {
                        proxy.get_manga_info(path.clone())
                    });
                }
                Command::GetChapters(source_id, path, tx) => {
                    process(&extension_map, source_id, tx, |proxy| {
                        proxy.get_chapters(path.clone())
                    });
                }
                Command::GetPages(source_id, path, tx) => {
                    process(&extension_map, source_id, tx, |proxy| {
                        proxy.get_pages(path.clone())
                    });
                }
            }
        }
    }
}

fn process<F, T>(extension_map: &HashMap<i64, Box<dyn Extension>>, source_id: i64, tx: Sender<T>, f: F)
where
    F: Fn(&Box<dyn Extension>) -> ExtensionResult<T>,
{
    match extension_map.get(&source_id) {
        Some(proxy) => {
            let res = f(proxy);
            if let Some(data) = res.data {
                if let Err(_) = tx.send(data) {
                    error!("receiver dropped");
                }
            }
        }
        None => {
            error!("extension with id {} not found", source_id);
        }
    }
}

fn wasi_read(env: &ExtensionEnv) -> Result<String, Box<dyn std::error::Error>> {
    let mut state = env.wasi_env.state();
    let wasm_stdout = state.fs.stdout_mut()?.as_mut().ok_or("no wasi stdout")?;
    let mut buf = String::new();
    wasm_stdout.read_to_string(&mut buf)?;
    Ok(buf)
}

fn wasi_write(
    env: &ExtensionEnv,
    param: &impl Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = env.wasi_env.state();
    let wasm_stdout = state.fs.stdin_mut()?.as_mut().ok_or("no wasi stdin")?;

    let buf = ron::to_string(param)?;
    wasm_stdout.write_all(&mut buf.as_bytes())?;

    Ok(())
}

fn host_http_request(env: &ExtensionEnv) {
    match do_http_request(env) {
        Ok(()) => {}
        Err(e) => {
            error!("error do_htp_request: {}", e);
        }
    }
}

fn do_http_request(env: &ExtensionEnv) -> Result<(), Box<dyn std::error::Error>> {
    let http_req_str = wasi_read(env)?;
    let http_req = ron::from_str::<Request>(&http_req_str)?;

    let mut req = ureq::get(&http_req.url);
    if let Some(headers) = http_req.headers {
        for (name, values) in headers {
            for value in values {
                req = req.set(&name, &value);
            }
        }
    }
    let res = req.call()?;
    let mut headers: HashMap<String, Vec<String>> = HashMap::new();
    for name in res.headers_names() {
        if let Some(header_value) = res.header(&name) {
            if let Some(header) = headers.get_mut(&name) {
                header.push(header_value.to_string());
            } else {
                headers.insert(name, vec![header_value.to_string()]);
            }
        }
    }

    let status = res.status() as i32;
    let body = res.into_string()?;

    let http_res = Response {
        headers,
        body,
        status,
    };

    wasi_write(env, &http_res)?;

    Ok(())
}

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, error::Error, path::PathBuf};
use tanoshi_lib::prelude::{Chapter, Extension, Manga, Param, Request, Response, Source};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot::Sender,
    },
    task::JoinHandle,
};
use wasmer::{imports, ChainableNamedResolver, Function, Instance, Module, Store, WasmerEnv};
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

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
        Self { extension_dir_path, tx }
    }

    pub async fn install(&self, name: String, contents: &Bytes) {
        let path = PathBuf::from(self.extension_dir_path.clone()).join(name).with_extension("wasm");
        std::fs::write(path.clone(), contents).unwrap();

        self.tx.send(Command::Load(path.to_str().unwrap().to_string())).unwrap();
    }

    pub async fn unload(&self, source_id: i64) {
        self.tx.send(Command::Unload(source_id)).unwrap()
    }

    pub async fn exist(&self, source_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Exist(source_id, tx)).unwrap();

        let exist = rx.await?;
        Ok(exist)
    }

    pub async fn detail(&self, source_id: i64) -> Result<Source, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx.send(Command::Detail(source_id, tx)).unwrap();

        let source = rx.await?;
        Ok(source)
    }

    pub async fn get_manga_list(
        &self,
        source_id: i64,
        param: Param,
    ) -> Result<Vec<Manga>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::GetMangaList(source_id, param, tx))
            .unwrap();

        let manga = rx.await?;
        Ok(manga)
    }

    pub async fn get_manga_info(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Manga, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::GetMangaInfo(source_id, path, tx))
            .unwrap();

        let manga = rx.await?;
        Ok(manga)
    }

    pub async fn get_chapters(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<Chapter>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::GetChapters(source_id, path, tx))
            .unwrap();

        let manga = rx.await?;
        Ok(manga)
    }

    pub async fn get_pages(
        &self,
        source_id: i64,
        path: String,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(Command::GetPages(source_id, path, tx))
            .unwrap();

        let manga = rx.await?;
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
    pub fn load(store: &Store, path: String) -> Self {
        let input = Pipe::new();
        let output = Pipe::new();
        let mut wasi_env = WasiState::new("tanoshi")
            .stdin(Box::new(input))
            .stdout(Box::new(output))
            .finalize()
            .unwrap();

        let extension_path = PathBuf::from(path.clone());

        let wasm_bytes = std::fs::read(extension_path).unwrap();
        let module = Module::new(&store, wasm_bytes).unwrap();

        let import_object = wasi_env.import_object(&module).unwrap();

        let env = ExtensionEnv { wasi_env };

        let tanoshi = imports! {
            "tanoshi" => {
                "host_http_request" => Function::new_native_with_env(&store, env.clone(), host_http_request)
            }
        };

        let instance = Instance::new(&module, &tanoshi.chain_back(import_object)).unwrap();

        ExtensionProxy {
            instance,
            path,
            env,
        }
    }

    fn call<T>(&self, name: &str) -> T
    where
        T: DeserializeOwned,
    {
        let res = self.instance.exports.get_function(name).unwrap();
        res.call(&[]).unwrap();
        let object_str = wasi_read(&self.env);
        ron::from_str(&object_str).unwrap()
    }

    fn call_with_args<T>(&self, name: &str, param: &impl Serialize) -> T
    where
        T: DeserializeOwned,
    {
        let res = self.instance.exports.get_function(name).unwrap();
        wasi_write(&self.env, &param);
        res.call(&[]).unwrap();
        let object_str = wasi_read(&self.env);
        ron::from_str(&object_str).unwrap()
    }
}

impl Extension for ExtensionProxy {
    fn detail(&self) -> Source {
        self.call("detail")
    }

    fn get_manga_list(&self, param: Param) -> tanoshi_lib::prelude::ExtensionResult<Vec<Manga>> {
        self.call_with_args("get_manga_list", &param)
    }

    fn get_manga_info(&self, path: String) -> tanoshi_lib::prelude::ExtensionResult<Manga> {
        self.call_with_args("get_manga_info", &path)
    }

    fn get_chapters(&self, path: String) -> tanoshi_lib::prelude::ExtensionResult<Vec<Chapter>> {
        self.call_with_args("get_chapters", &path)
    }

    fn get_pages(&self, path: String) -> tanoshi_lib::prelude::ExtensionResult<Vec<String>> {
        self.call_with_args("get_pages", &path)
    }
}

pub fn start() -> (JoinHandle<()>, UnboundedSender<Command>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async {
        extension_thread(rx).await;
    });

    (handle, tx)
}

pub async fn load(extention_dir_path: String, tx: UnboundedSender<Command>) {
    match std::fs::read_dir(extention_dir_path.clone()) {
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::create_dir_all(extention_dir_path.clone());
        }
    }

    for entry in std::fs::read_dir(extention_dir_path.clone())
        .unwrap()
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
        let path = entry.unwrap().path();
        info!("load plugin from {:?}", path.clone());
        tx.send(Command::Load(path.to_str().unwrap().to_string()))
            .unwrap();
    }
}

async fn extension_thread(extension_receiver: UnboundedReceiver<Command>) {
    let mut recv = extension_receiver;
    let mut extension_map = HashMap::new();

    let store = Store::default();

    loop {
        let cmd = recv.recv().await;
        if let Some(cmd) = cmd {
            match cmd {
                Command::Load(path) => {
                    let proxy = ExtensionProxy::load(&store, path);
                    let source = proxy.detail();
                    info!("loaded: {:?}", source);

                    extension_map.insert(source.id, proxy);
                }
                Command::Unload(source_id) => {
                    extension_map.remove(&source_id);
                }
                Command::Exist(source_id, tx) => {
                    let exist = extension_map.get(&source_id).is_some();
                    tx.send(exist).unwrap();
                }
                Command::Detail(source_id, tx) => {
                    let proxy = extension_map.get(&source_id).unwrap();
                    let res = proxy.detail();
                    tx.send(res).unwrap();
                }
                Command::GetMangaList(source_id, param, tx) => {
                    let proxy = extension_map.get(&source_id).unwrap();
                    let res = proxy.get_manga_list(param);
                    tx.send(res.data.unwrap()).unwrap();
                }
                Command::GetMangaInfo(source_id, path, tx) => {
                    let proxy = extension_map.get(&source_id).unwrap();
                    let res = proxy.get_manga_info(path);
                    tx.send(res.data.unwrap()).unwrap();
                }
                Command::GetChapters(source_id, path, tx) => {
                    let proxy = extension_map.get(&source_id).unwrap();
                    let res = proxy.get_chapters(path);
                    tx.send(res.data.unwrap()).unwrap();
                }
                Command::GetPages(source_id, path, tx) => {
                    let proxy = extension_map.get(&source_id).unwrap();
                    let res = proxy.get_pages(path);
                    tx.send(res.data.unwrap()).unwrap();
                }
            }
        }
    }
}

fn wasi_read(env: &ExtensionEnv) -> String {
    let mut state = env.wasi_env.state();
    let wasm_stdout = state.fs.stdout_mut().unwrap().as_mut().unwrap();
    let mut buf = String::new();
    wasm_stdout.read_to_string(&mut buf).unwrap();
    buf
}

fn wasi_write(env: &ExtensionEnv, param: &impl Serialize) {
    let mut state = env.wasi_env.state();
    let wasm_stdout = state.fs.stdin_mut().unwrap().as_mut().unwrap();

    let buf = ron::to_string(param).unwrap();
    wasm_stdout.write_all(&mut buf.as_bytes()).unwrap()
}

fn host_http_request(env: &ExtensionEnv) {
    let http_req_str = wasi_read(env);
    let http_req = ron::from_str::<Request>(&http_req_str).unwrap();

    let mut req = ureq::get(&http_req.url);
    if let Some(headers) = http_req.headers {
        for (name, values) in headers {
            for value in values {
                req = req.set(&name, &value);
            }
        }
    }
    let res = req.call().unwrap();

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
    let body = res.into_string().unwrap();

    let http_res = Response {
        headers,
        body,
        status,
    };

    wasi_write(env, &http_res);
}

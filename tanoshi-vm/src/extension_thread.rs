use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, fmt::Debug, path::PathBuf};
use tanoshi_lib::prelude::{
    Chapter, Extension, ExtensionResult, Filters, Manga, Param, Request, Response, Source,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use wasmer::{
    imports, ChainableNamedResolver, Function, Instance, Module, Store, Universal, WasmerEnv,
};
use wasmer_compiler_cranelift::Cranelift;
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use crate::extension_bus::{Command, ExtensionResultSender};

#[derive(WasmerEnv, Clone)]
struct ExtensionEnv {
    wasi_env: WasiEnv,
}

struct ExtensionProxy {
    instance: Instance,
    env: ExtensionEnv,
}

impl ExtensionProxy {
    pub fn load(
        store: &Store,
        path: String,
    ) -> Result<Box<dyn Extension>, Box<dyn std::error::Error>> {
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

        Ok(Box::new(ExtensionProxy { instance, env }))
    }

    fn call<T>(&self, name: &str) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let res = self.instance.exports.get_function(name)?;
        res.call(&[])?;
        let object_str = wasi_read(&self.env)?;
        debug!("call {} => {}", name, object_str);
        Ok(ron::from_str(&object_str)?)
    }

    fn call_with_args<T, U>(&self, name: &str, param: &U) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
        U: Serialize + Debug,
    {
        let res = self.instance.exports.get_function(name)?;
        if let Err(e) = wasi_write(&self.env, &param) {
            error!("error write to wasi: {}", e);
        }
        res.call(&[])?;
        let object_str = wasi_read(&self.env)?;
        debug!("call {}({:?}) => {}", name, param, object_str);
        Ok(ron::from_str(&object_str)?)
    }
}

impl Extension for ExtensionProxy {
    fn detail(&self) -> Source {
        self.call("detail").unwrap_or_default()
    }

    fn filters(&self) -> ExtensionResult<Option<Filters>> {
        match self.call("filters") {
            Ok(res) => res,
            Err(e) => ExtensionResult::err(format!("{}", e).as_str()),
        }
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

pub fn start() -> (JoinHandle<()>, UnboundedSender<Command>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async {
        thread(rx).await;
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

async fn thread(extension_receiver: UnboundedReceiver<Command>) {
    let mut recv = extension_receiver;
    let mut extension_map: HashMap<i64, Box<dyn Extension>> = HashMap::new();

    let compiler = Cranelift::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);

    loop {
        let cmd = recv.recv().await;
        if let Some(cmd) = cmd {
            match cmd {
                Command::Insert(source_id, proxy) => {
                    extension_map.insert(source_id, proxy);
                }
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
                Command::List(tx) => {
                    let sources = extension_map
                        .iter()
                        .map(|(_, ext)| ext.detail())
                        .collect::<Vec<Source>>();

                    if let Err(_) = tx.send(sources) {
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
                Command::Filters(source_id, tx) => {
                    process(&extension_map, source_id, tx, |proxy| proxy.filters());
                }
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

fn process<F, T>(
    extension_map: &HashMap<i64, Box<dyn Extension>>,
    source_id: i64,
    tx: ExtensionResultSender<T>,
    f: F,
) where
    F: Fn(&Box<dyn Extension>) -> ExtensionResult<T>,
{
    match extension_map.get(&source_id) {
        Some(proxy) => {
            let res = f(proxy);
            if let Err(_) = tx.send(res) {
                error!("receiver dropped");
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

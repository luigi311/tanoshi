use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, fmt::Debug, path::Path};
use tanoshi_lib::prelude::{
    Chapter, Extension, ExtensionResult, Filters, Manga, Param, Request, Response, Source,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use wasmer::{imports, ChainableNamedResolver, Function, Instance, Module, Store, WasmerEnv};

use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use crate::bus::{Command, ExtensionResultSender};

#[derive(WasmerEnv, Clone)]
struct ExtensionEnv {
    wasi_env: WasiEnv,
}

struct ExtensionProxy {
    instance: Instance,
    env: ExtensionEnv,
}

impl ExtensionProxy {
    pub fn load<P: AsRef<Path>>(
        store: &Store,
        path: P,
    ) -> Result<Box<dyn Extension>, Box<dyn std::error::Error>> {
        let module = unsafe { Module::deserialize_from_file(&store, path)? };

        let input = Pipe::new();
        let output = Pipe::new();

        let mut wasi_env = WasiState::new("tanoshi")
            .stdin(Box::new(input))
            .stdout(Box::new(output))
            .finalize()?;

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

    #[cfg(not(feature = "disable-compiler"))]
    fn init_store() -> Store {
        #[cfg(feature = "cranelift")]
        let compiler = wasmer_compiler_cranelift::Cranelift::new();
        #[cfg(all(feature = "llvm", not(feature = "cranelift")))]
        let compiler = wasmer_compiler_llvm::LLVM::new();

        #[cfg(feature = "universal")]
        let engine = wasmer::Universal::new(compiler).engine();
        #[cfg(all(feature = "dylib", not(feature = "universal")))]
        let engine = wasmer_engine_dylib::Dylib::new(compiler).engine();

        Store::new(&engine)
    }

    fn init_store_headless() -> Store {
        #[cfg(feature = "dylib")]
        let engine = wasmer_engine_dylib::Dylib::headless().engine();
        #[cfg(feature = "universal")]
        let engine = wasmer::Universal::headless().engine();

        Store::new(&engine)
    }

    #[cfg(not(feature = "disable-compiler"))]
    pub fn compile_from_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let wasm_bytes = std::fs::read(&path)?;

        let output = std::path::PathBuf::new()
            .join(&path)
            .with_extension("tanoshi");
        Self::compile(&wasm_bytes, output)?;

        std::fs::remove_file(path)?;
        Ok(())
    }

    #[cfg(not(feature = "disable-compiler"))]
    pub fn compile<P: AsRef<Path>>(
        wasm_bytes: &[u8],
        output: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let store = Self::init_store();

        debug!("compiling extension");
        let module = Module::new(&store, wasm_bytes)?;
        debug!("done");

        debug!("remove wasm file");
        Ok(module.serialize_to_file(output)?)
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

pub async fn load<P: AsRef<Path>>(
    path: P,
    tx: UnboundedSender<Command>,
) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::read_dir(&path) {
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::create_dir_all(&path);
        }
    }
    
    #[cfg(not(feature = "disable-compiler"))]
    compile(&path).await?;

    for entry in std::fs::read_dir(&path)?
        .into_iter()
        .filter_map(Result::ok)
        .filter(move |path| {
            path.path()
                .extension()
                .map_or(false, |ext| ext == "tanoshi")
        })
    {
        let path = entry.path();
        info!("found compiled plugin at {:?}", path.clone());
        tx.send(Command::Load(
            path.to_str().ok_or("no path str")?.to_string(),
        ))?;
    }

    Ok(())
}

#[cfg(not(feature = "disable-compiler"))]
pub async fn compile<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::read_dir(&path) {
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::create_dir_all(&path);
        }
    }

    for entry in std::fs::read_dir(&path)?
        .into_iter()
        .filter_map(Result::ok)
        .filter(move |path| path.path().extension().map_or(false, |ext| ext == "wasm"))
    {
        let path = entry.path();
        info!("found wasm file at {:?}", path.clone());
        ExtensionProxy::compile_from_file(path).map_err(|e| format!("{}", e))?;
    }

    Ok(())
}

async fn thread(extension_receiver: UnboundedReceiver<Command>) {
    let mut recv = extension_receiver;
    let mut extension_map: HashMap<i64, Box<dyn Extension>> = HashMap::new();

    let store = ExtensionProxy::init_store_headless();

    loop {
        let cmd = recv.recv().await;
        if let Some(cmd) = cmd {
            match cmd {
                Command::Insert(source_id, proxy) => {
                    extension_map.insert(source_id, proxy);
                }
                Command::Load(path) => {
                    info!("load plugin from {:?}", path.clone());
                    match ExtensionProxy::load(&store, path) {
                        Ok(proxy) => {
                            let source = proxy.detail();
                            info!("loaded: {:?}", source);
                            extension_map.insert(source.id, proxy);
                        }
                        Err(e) => {
                            error!("error load extension: {}", e);
                        }
                    }
                }
                Command::Unload(source_id) => {
                    extension_map.remove(&source_id);
                }
                Command::Exist(source_id, tx) => {
                    let exist = extension_map.get(&source_id).is_some();
                    if tx.send(exist).is_err() {
                        error!("receiver dropped");
                    }
                }
                Command::List(tx) => {
                    let sources = extension_map
                        .iter()
                        .map(|(_, ext)| ext.detail())
                        .collect::<Vec<Source>>();

                    if tx.send(sources).is_err() {
                        error!("receiver dropped");
                    }
                }
                Command::Detail(source_id, tx) => match extension_map.get(&source_id) {
                    Some(proxy) => {
                        let res = proxy.detail();
                        if tx.send(res).is_err() {
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
            if tx.send(res).is_err() {
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
    wasm_stdout.write_all(buf.as_bytes())?;

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

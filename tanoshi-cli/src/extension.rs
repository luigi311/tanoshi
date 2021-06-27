use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tanoshi_lib::prelude::{Chapter, Extension, Manga, Param, Request, Response, SortByParam, SortOrderParam, Source};
use wasmer::{imports, ChainableNamedResolver, Function, Instance, Module, Store, WasmerEnv};
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

#[derive(WasmerEnv, Clone)]
struct ExtensionEnv {
    wasi_env: WasiEnv,
}

pub struct ExtensionProxy {
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

pub fn load(extention_dir_path: String) -> HashMap<i64, ExtensionProxy> {
    let mut extension_map = HashMap::new();

    let store = Store::default();

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
        let entry = entry.unwrap();
        let path = entry.path().to_str().unwrap().to_string();
        let file_name = entry.path().file_name().unwrap().to_str().unwrap().to_string();
        let new_path = format!("repo/library/{}", file_name);

        let _ = std::fs::copy(path, &new_path);

        info!("load plugin from {:?}", new_path.clone());
        let proxy = ExtensionProxy::load(&store, new_path.clone());
        let source = proxy.detail();
        info!("loaded: {:?}", source);

        extension_map.insert(source.id, proxy);
    }

    extension_map
}

pub fn test(path: String) {
    let mut extension_map = HashMap::new();

    let store = Store::default();
    for entry in std::fs::read_dir(path.clone())
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
        let entry = entry.unwrap();
        let path = entry.path().to_str().unwrap().to_string();
        let proxy = ExtensionProxy::load(&store, path.clone());
        let source = proxy.detail();
        println!("loaded: {:?}", source);

        extension_map.insert(source.id, proxy);
    }

    for (_, ext) in extension_map {
        let detail = ext.detail();
        println!("Test {}", detail.name);
        
        let param = Param{
            keyword: None,
            genres: None,
            page: None,
            sort_by: Some(SortByParam::LastUpdated),
            sort_order: Some(SortOrderParam::Desc),
            auth: None,
        };

        print!("Test get_manga_list ");
        let res = ext.get_manga_list(param);
        if let Some(res) = res.error {
            println!("Error {}", res);
            return;
        }
        println!("ok");
        
        let manga = res.data.unwrap();
        print!("Test get_manga_info {} ", manga[0].path.clone());
        let res = ext.get_manga_info(manga[0].path.clone());
        if let Some(res) = res.error {
            println!("Error {}", res);
            return;
        }
        println!("ok");

        print!("Test get_chapters {} ", manga[0].path.clone());
        let res = ext.get_chapters(manga[0].path.clone());
        if let Some(res) = res.error {
            println!("Error {}", res);
            return;
        }
        println!("ok");

        let chapters = res.data.unwrap();
        print!("Test get_pages {} ", chapters[0].path.clone());
        let res = ext.get_pages(chapters[0].path.clone());
        if let Some(res) = res.error {
            println!("Error {}", res);
            return;
        }
        println!("ok");
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
use std::io;

use serde::{de::DeserializeOwned, Serialize};

use crate::data::{Request, Response};

pub fn http_request(req: Request) -> Response {
    write_object(req);
    unsafe {
        host_http_request();
    }
    read_object()
}

pub fn write_object<T: Serialize>(data: T) {
    let serialized = ron::to_string(&data).unwrap();
    println!("{}", serialized)
}

pub fn read_object<T: DeserializeOwned>() -> T {
    let mut serialized = String::new();
    io::stdin().read_line(&mut serialized).unwrap();
    ron::from_str(&serialized).unwrap()
}

#[link(wasm_import_module = "tanoshi")]
extern "C" {
    fn host_http_request();
}

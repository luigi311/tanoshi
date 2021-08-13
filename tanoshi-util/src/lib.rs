use std::collections::HashMap;

use tanoshi_lib::data::{Request, Response};

pub fn http_request(req: Request) -> Response {
    if let Err(err) = tanoshi_lib::shim::write_object(req) {
        return Response {
            headers: HashMap::new(),
            body: format!("{}", err),
            status: 9999,
        };
    }

    unsafe { host_http_request() };
    tanoshi_lib::shim::read_object().unwrap_or_else(|err| Response {
        headers: HashMap::new(),
        body: format!("{}", err),
        status: 9999,
    })
}

#[link(wasm_import_module = "tanoshi")]
extern "C" {
    fn host_http_request();
}

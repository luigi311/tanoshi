use tanoshi_lib::data::{Request, Response};

pub fn http_request(req: Request) -> Response {
    tanoshi_lib::shim::write_object(req);
    unsafe { host_http_request() };
    tanoshi_lib::shim::read_object()
}

#[link(wasm_import_module = "tanoshi")]
extern "C" {
    fn host_http_request();
}
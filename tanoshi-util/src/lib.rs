use std::collections::HashMap;

use tanoshi_lib::data::{Request, Response};

#[cfg(not(feature = "__test"))]
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

#[cfg(not(feature = "__test"))]
#[link(wasm_import_module = "tanoshi")]
extern "C" {
    fn host_http_request();
}

#[cfg(feature = "__test")]
pub fn http_request(req: Request) -> Response {
    let client = reqwest::blocking::Client::new();
    if req.method == "GET" {
        let mut request_builder = client.get(req.url);

        if let Some(headers) = req.headers.as_ref() {
            for (key, values) in headers {
                for value in values {
                    request_builder = request_builder.header(key, value);
                }
            }
        }

        match request_builder.send() {
            Ok(response) => {
                let status = response.status();
                Response {
                    headers: {
                        let header_map = response.headers();
                        header_map
                            .keys()
                            .map(|key| {
                                (
                                    key.to_string(),
                                    header_map
                                        .get_all(key)
                                        .iter()
                                        .flat_map(|value| value.to_str().ok().map(str::to_string))
                                        .collect(),
                                )
                            })
                            .collect()
                    },
                    body: response.text().unwrap_or_else(|_| "".to_string()),
                    status: status.as_u16() as i32,
                }
            }
            Err(err) => Response {
                headers: HashMap::new(),
                body: format!("{}", err),
                status: 9999,
            },
        }
    } else {
        Response {
            headers: HashMap::new(),
            body: "only GET requests are supported at the moment".to_string(),
            status: 9999,
        }
    }
}

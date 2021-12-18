export class Response {
    constructor(readonly headers: Map<string, string>, readonly body: Uint8Array) { }

    json(): any {
        var string = this.text();
        return JSON.parse(string);
    }

    text(): string {
        // @ts-ignore: Unreachable code error
        var string = bytes_to_string(this.body);
        return string;
    }
}

export async function fetch(url: string, options?: {
    method?: string,
    headers?: Map<string, string>,
}): Promise<Response> {
    // @ts-ignore: Unreachable code error
    let res = await __native_fetch__(url, options);
    return Promise.resolve(new Response(res.headers, res.body));
}
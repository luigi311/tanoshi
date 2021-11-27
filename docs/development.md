# Development

## Build
Tanoshi backend use [rust-embed](https://github.com/pyros2097/rust-embed) to embed static files to the binary. Because of this, `tanoshi-web` need to be built first so `tanoshi` will be able to build successfully.

### Frontend
1. Install Rust
2. Install trunk and wasm-bindgen-cli
   ```
   cargo install trunk wasm-bindgen-cli
   ```
3. Change directory into `tanoshi-web`
    ```
    cd crates/tanoshi-web 
    ```
3. Build
    ```
    trunk build
    ```

### Backend
1. Change directory into `crates/tanoshi` or root repository
2. Install dependencies for https://github.com/faldez/libarchive-rs
3. Install dependency for https://gitlab.com/taricorp/llvm-sys.rs
   - on Windows, you can download from https://github.com/faldez/tanoshi-builder/releases/download/v0.1.0/LLVM.7z extract to a directory and set environment variable `$LLVM_SYS_110_PREFIX` to `/path/to/llvm` or build it yourself
   - on macOS, install using homebrew `brew install llvm@11` ands set `LLVM_SYS_110_PREFIX` to `/usr/local/opt/llvm`
   - on Linux
        ```bash
        wget https://apt.llvm.org/llvm.sh 
        chmod +x llvm.sh
        ./llvm.sh 11
        ```
4. Build
    ```
    cargo build
    ```

### Desktop
1. Do steps for both frontend and backend
2. Install depedencies for tauri
3. Install tauri cli
   ```
   cargo install tauri-cli --version ^1.0.0-beta
   ```
4. Run
   ```
   cd crates/tanoshi-web
   tauri serve
   
   cd crates/tanoshi
   cargo tauri dev
   ```

PS. On linux you may need to install libssl-dev on ubuntu/debian or openssl-dev on fedora/centos
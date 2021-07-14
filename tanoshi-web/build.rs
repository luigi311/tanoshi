fn main() {
    #[cfg(not(debug_assertions))]
    npm_rs::NpmEnv::default()
        .init()
        .install(None)
        .run("build")
        .exec()
        .expect("failed to build");
}

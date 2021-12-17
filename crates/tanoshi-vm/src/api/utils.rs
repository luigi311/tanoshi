use rquickjs::bind;

#[bind(object, public)]
#[quickjs(rename = "__native_print__")]
pub fn print(msg: String) {
    println!("{}", msg);
}

#[bind(object, public)]
mod console {
    pub fn log(args: String) {
        println!("{}", args);
    }
    pub fn info(args: String) {
        info!("{}", args);
    }
    pub fn error(args: String) {
        error!("{}", args);
    }
    pub fn debug(args: String) {
        debug!("{}", args);
    }
}

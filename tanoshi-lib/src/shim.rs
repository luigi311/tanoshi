
use std::io;

use serde::{de::DeserializeOwned, Serialize};

pub fn write_object<T: Serialize>(data: T) {
    let serialized = ron::to_string(&data).unwrap();
    println!("{}", serialized)
}

pub fn read_object<T: DeserializeOwned>() -> T {
    let mut serialized = String::new();
    io::stdin().read_line(&mut serialized).unwrap();
    ron::from_str(&serialized).unwrap()
}


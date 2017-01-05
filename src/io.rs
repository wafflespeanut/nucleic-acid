use bincode::SizeLimit;
use bincode::rustc_serialize as serializer;
use rustc_serialize::{Decodable, Encodable};

use std::fs::{self, File};

pub fn write<T: Encodable + Decodable>(path: &str, obj: &T) {
    let mut fd = File::create(path).unwrap();
    serializer::encode_into(&obj, &mut fd, SizeLimit::Infinite).unwrap();
}

pub fn read<T: Encodable + Decodable>(path: &str) -> T {
    let mut fd = File::open(path).unwrap();
    let data = serializer::decode_from(&mut fd, SizeLimit::Infinite).unwrap();
    fs::remove_file(path).unwrap();
    data
}

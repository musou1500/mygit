use flate2::read::ZlibDecoder;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::io::stdout;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();
    let hash = &args[1];
    let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    let mut reader = std::io::BufReader::new(ZlibDecoder::new(fs::File::open(path).unwrap()));
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();
    stdout().write(buf.as_slice()).unwrap();
}

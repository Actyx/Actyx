fn main() {
    prost_build::compile_protos(&["src/unixfsv1/unixfs.proto"], &["src"]).unwrap();
}

fn main() {
  if cfg!(test) {
    println!("cargo:rerun-if-changed=tests/data");
  }
}

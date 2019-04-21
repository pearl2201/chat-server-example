fn main() {
    println!("cargo:rustc-flags=-l framework=linux/input");
    println!("cargo:rustc-flags=-l framework=linux/notifier");
}
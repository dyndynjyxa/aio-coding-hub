fn main() {
    if let Err(error) =
        aio_coding_hub_lib::export_typescript_bindings("../src/generated/bindings.ts")
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

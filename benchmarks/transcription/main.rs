fn main() {
    if let Err(error) = lyra_lib::regression_bench::run(std::env::args().skip(1).collect()) {
        eprintln!("benchmark: {error}");
        std::process::exit(1);
    }
}

//! The `sisu` executable.

fn main() {
    // Internal errors are reported by the driver as a catalog message;
    // the default hook's backtrace would only repeat it.
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut host = sisu_interp::StdHost::new(Vec::new());
    let code = sisu_driver::cli::execute(&args, &mut host);
    std::process::exit(code);
}

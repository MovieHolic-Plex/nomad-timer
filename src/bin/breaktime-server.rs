fn main() -> std::io::Result<()> {
    let addr = std::env::var("BREAKTIME_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:17878".into());
    breaktime::server::run_server(&addr)
}

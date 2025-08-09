use clap::Parser;

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct ServerArgs {
    #[arg(short, long, default_value_t = 10080)]
    port: u16,
    #[arg(long, default_value_t = DEFAULT_HOST.to_string())]
    host: String,
    #[arg(short, long, default_value_t = 60)]
    worker: u32,
    #[arg(short, long, default_value_t = 4)]
    reserve: u32,
}

#[derive(Debug)]
pub struct Server {
    config: ServerArgs,
}

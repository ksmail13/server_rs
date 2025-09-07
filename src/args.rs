use std::rc::Rc;

use clap::Parser;

use crate::process::process::Process;

const DEFAULT_HOST: &str = "0.0.0.0";

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[arg(short, long, default_value_t = 10080)]
    pub port: u16,
    #[arg(long, default_value_t = 10079)]
    pub reserve_port: u16,
    #[arg(long, default_value_t = DEFAULT_HOST.to_string())]
    pub host: String,
    #[arg(short, long, default_value_t = 60)]
    pub worker: u32,
    #[arg(short, long, default_value_t = 4)]
    pub reserve: u32,
    #[arg(short, long, default_value_t = 500)]
    pub timeout_ms: u64,
}

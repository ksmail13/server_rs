mod error;
pub mod group;
mod helper;
pub mod manager;

pub trait Worker {
    fn run(&self);
}

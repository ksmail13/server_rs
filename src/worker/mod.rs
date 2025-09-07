pub mod group;
mod helper;
pub mod manager;
mod worker;
pub trait Worker {
    fn run(&self);
}

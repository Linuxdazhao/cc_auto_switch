use anyhow::Result;
use std::path::PathBuf;

pub struct Pidfile {
    _path: PathBuf,
}

impl Pidfile {
    pub fn new(path: PathBuf) -> Self {
        Self { _path: path }
    }
    pub fn acquire(&self) -> Result<()> {
        unimplemented!("Task 4")
    }
    pub fn release(&self) -> Result<()> {
        unimplemented!("Task 4")
    }
    pub fn read(&self) -> Result<Option<u32>> {
        unimplemented!("Task 4")
    }
}

pub fn process_alive(_pid: u32) -> Result<bool> {
    unimplemented!("Task 4")
}
pub fn process_name(_pid: u32) -> Option<String> {
    unimplemented!("Task 4")
}

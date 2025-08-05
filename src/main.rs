use anyhow::Result;

mod cmd;

fn main() -> Result<()> {
    cmd::main::run()
}

use std::path::PathBuf;
use std::process::{Child, Command};

pub fn spawn_chrome(chrome: &PathBuf)-> Result<Child, Box<dyn std::error::Error>> {
    let child_process = Command::new(chrome).arg("--port=6969").spawn()?;
    Ok(child_process)
}

pub fn kill_chrome(mut chrome: Child) -> Result<(), Box<dyn std::error::Error>> {
    chrome.kill()?;
    Ok(())
}
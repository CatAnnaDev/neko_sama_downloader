use std::{
    path::PathBuf,
    process::{Child, Command},
};

pub struct ChromeChild {
    pub chrome: Child,
}

impl ChromeChild {
    pub fn spawn(chrome: &PathBuf) -> Self {
        let child_process = Command::new(chrome)
            .args([
                "--port=6969",
                "--ignore-certificate-errors",
                "--disable-logging",
                "--disable-in-process-stack-traces",
                "--no-sandbox",
                "--silent",
            ])
            .spawn()
            .expect("Can't spawn chromeDriver");
        ChromeChild {
            chrome: child_process,
        }
    }
}

impl Drop for ChromeChild {
    fn drop(&mut self) {
        self.chrome.kill().unwrap();
    }
}

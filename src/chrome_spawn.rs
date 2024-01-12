use std::path::PathBuf;
use std::process::{Command, exit};
use std::sync::{Arc, Mutex};
use std::thread;
use ctrlc::set_handler;
use crossbeam_channel::unbounded;

pub fn spawn_chrome(chrome: &PathBuf){
    let (tx, rx) = unbounded::<()>();
    let running = Arc::new(Mutex::new(true));

    let running_clone = Arc::clone(&running);

    set_handler(move || {
        if let Ok(mut running) = running_clone.lock() {
            *running = false;
        }
        if let Ok(_) = tx.send(()){

        }
    }).expect("set_handler Error");

    let mut child = Command::new(chrome).arg("--port=6969").spawn().expect("Can't start chromedriver");

    thread::spawn(move || {
        while *running.lock().unwrap() {
            if let Ok(_) = rx.try_recv() {
                child.kill().unwrap();
                exit(0);
            }
        }
    });
}
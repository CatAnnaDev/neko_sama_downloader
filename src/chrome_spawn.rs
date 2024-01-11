use std::path::PathBuf;
use std::process::{Command, exit, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use ctrlc::set_handler;
use crossbeam_channel::unbounded;

pub fn spawn_chrome(chrome: &PathBuf){
    let (tx, rx) = unbounded::<()>();
    let running = Arc::new(Mutex::new(true));

    let running_clone = Arc::clone(&running);

    set_handler(move || {
        let mut running = running_clone.lock().unwrap();
        *running = false;
        if let Ok(_) = tx.send(()){

        }
    }).expect("set_handler Error");

    println!("spawn chrome");
    let child = Command::new(chrome)
        .args([
            "--ignore-certificate-errors",
            "--disable-logging",
            "--disable-logging-redirect",
            "--port=6969",
        ])
        .stdout(Stdio::null())
        .spawn();

    thread::spawn(move || {
        while *running.lock().unwrap() {
            if let Ok(_) = rx.try_recv() {
                child.unwrap().kill().unwrap();
                exit(0);
            }
        }
    });
}
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("Parent PID is {}", std::process::id());

    ctrlc::set_handler(move || {
        println!("received interrupt!");
    })
    .expect("Error setting Ctrl-C handler");

    let mut cmd = Command::new("/opt/homebrew/bin/zsh");

    // When I run this, I can't `ctrl-c` the program anymore:
    cmd.args(&["-l", "-i", "-c", "/usr/bin/env -0; exit 0;"]);

    // But if it's this, I can `ctrl-c`:
    // cmd.args(&["-l", "-i", "-c", "/usr/bin/env -0; exit 0"]);
    //
    // WHY?!
    //
    // I just launch a subprocess that exits. How can it hijack my signal handling?

    let output = cmd.output().expect("Failed to execute command");

    println!("child status: {}", &output.status);

    println!("Try to hit ctrl-c to exit the program");
    loop {
        thread::sleep(Duration::from_secs(1));
        println!("still running...");
    }
}

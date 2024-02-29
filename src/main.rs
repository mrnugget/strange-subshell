use libc;
use std::os::unix::process::CommandExt; // for pre_exec
use std::process::{self, Command};
use std::thread;
use std::time::Duration; // ensure you've added libc to your dependencies

fn main() {
    println!("Parent PID is {}", std::process::id());
    println!("Parent Process Group ID is {:?}", rustix::process::getgid());
    let stty_before = get_stty_settings();

    ctrlc::set_handler(move || {
        println!("received interrupt!");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let mut cmd = Command::new("/bin/zsh");
    // When I run this, I can't `ctrl-c` the program anymore:
    cmd.args(&[
        "-i",       // interactive shell, because that's what we want
        "--no-rcs", // no rc files
        "-f",       // no rc files, again, for good measure
        "-d",       // no global rc files,
        "-c", "ls",
    ]);

    // But if it's this, I can `ctrl-c`:
    // cmd.args(&["-l", "-i", "-c", "ls; echo FOOBAR"]);
    //
    // WHY?!
    //
    // I just launch a subprocess that exits. How can it hijack my signal handling?

    // BUT THIS FIXES IT!! When I set a new session ID on the sub process, it doesn't seem to take over control
    // of my terminal.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let output = cmd.output().expect("Failed to execute command");
    println!("child exited with status: {}", &output.status);

    let stty_after = get_stty_settings();
    if stty_before != stty_after {
        println!("stty settings changed!");
        println!("before: {}", stty_before);
        println!("after: {}", stty_after);
    }

    println!(
        "AFTER running the process. Parent Process Group ID is {:?}",
        rustix::process::getgid()
    );
    println!("Try to hit ctrl-c to exit the program");
    let mut sleeps = 0;
    loop {
        thread::sleep(Duration::from_secs(1));
        println!("Still running.... Hit ctrl-c to try to exit the program. Or send INT manually: `kill -2 {}`", std::process::id());

        sleeps += 1;
        if sleeps > 10 {
            println!("alright, exiting now on my own.");
            break;
        }
    }
}

fn get_stty_settings() -> String {
    let output = Command::new("stty")
        .args(&["-a"])
        .stdin(process::Stdio::inherit())
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).to_string()
}

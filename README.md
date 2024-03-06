# Why does a subprocess hijack my `Ctrl-c`?

When I spawn a subprocess that runs `$SHELL -i -c 'ls'` and exits (!)
I can't send the interrupt signal via `Ctrl-c` to my process anymore.

Why?

It only happens when the command isn't followed by another command.

This hijacks `Ctrl-c`:

```rust
let mut cmd = Command::new("/opt/homebrew/bin/zsh");
// When I run this, I can't `ctrl-c` the program anymore:
cmd.args(&["-i", "-c", "ls"]);
let _ = cmd.output().expect("Failed to execute command");
```

And these do _NOT_ hijack `Ctrl-c`:

```rust
// Ctrl-c works after these:
cmd.args(&["-i", "-c", "ls; echo FOOBAR"]);
cmd.args(&["-i", "-c", "ls; exit 0"]);
```

Why?!

## Findings so far

- `stty -a` is unchanged before and after
- Process can still receive interrupt signal (`kill -INT`) and exits
- It's only `Ctrl-c` that doesn't work anymore.
- It happens for me with `/bin/zsh` (the macOS built-in one) and the homebrew `zsh`
- It does _NOT_ happen with `/bin/bash` (!!!)
- It does _NOT_ happen with `/opt/homebrew/bin/fish` (!!!)
- It happens even if I run `zsh` with `--no-rcs` and/or comment out my `.zprofile`/`.zshrc`/`.profile` completely. I can run ZSH without any config files (`-f` to disable files and `-d` to disable global config files) and it still happens.
- It's not just `Ctrl-c` that doesn't work anymore: `ctrl-z` (suspend) and `ctrl-\` (quit) also don't work anymore.

Big breakthrough: when I use the following to set a new session on the spawned shell process, `ctrl-c` still works:

```rust
unsafe {
    cmd.pre_exec(|| {
        if libc::setsid() == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    });
}
```

So it is related to session IDs!

GPT4 thinks this helps, because:

> When zsh executes a non-builtin command (like `ls` in your example), it typically forks a child process for the command and may create a new process group for this child, especially in interactive mode or when job control is involved. zsh, as the parent of this command, can become the process group leader for the command it executes.
> As the process group leader, zsh can receive signals meant for the group. This includes SIGINT (triggered by ctrl-c) intended to interrupt the current foreground job.
> The terminal is associated with a foreground process group. When ctrl-c is pressed, the terminal sends SIGINT to the foreground process group.
> If `zsh` changes the foreground process group of the terminal to its own new group for the command it's executing, then SIGINT would go to zsh and its child processes, not your Rust program.
> In theory, when zsh exits, it should clean up, restoring the original process group as the terminal's foreground group. However, the intricacies of how zsh handles exit and cleanup, especially after launching interactive sessions or commands, might not always revert all changes perfectly, particularly in how signals are handled or how the terminal's foreground process group is managed.


- This is also confirmed when we print the foreground process group ID!
    - With `ls;` it's changed and we can't send signals, because signals are
      sent to the foreground process group ID.
    - With `ls; exit 1` it gets reset back

- In `strace` we can see that the difference is that when `zsh` runs a command
  as the last one in `-c`, it doesn't restore anything. It `execve`s into `ls`
  itself!

- In the "good" strace (when we run `zsh -c 'ls; exit 0'` we can see that zsh
  restores the process group id with `setpgid`.

  Here is the end of `ls;`:

    [pid 12760] write(1, "Cargo.lock\nCargo.toml\nREADME.md\n"..., 58) = 58
    [pid 12760] close(1 <unfinished ...>
    [pid 12759] <... poll resumed>)         = 1 ([{fd=4, revents=POLLIN}])
    [pid 12760] <... close resumed>)        = 0
    [pid 12759] read(4,  <unfinished ...>
    [pid 12760] close(2 <unfinished ...>
    [pid 12759] <... read resumed>"Cargo.lock\nCargo.toml\nREADME.md\n", 32) = 32
    [pid 12760] <... close resumed>)        = 0
    [pid 12759] read(4, "src\nstrace-bad.txt\ntarget\n", 32) = 26
    [pid 12760] exit_group(0 <unfinished ...>
    [pid 12759] read(4,  <unfinished ...>
    [pid 12760] <... exit_group resumed>)   = ?
    [pid 12759] <... read resumed>"", 6)    = 0
    [pid 12759] ioctl(6, FIONBIO, [0])      = 0
    [pid 12759] read(6, "", 32)             = 0
    [pid 12759] close(6)                    = 0
    [pid 12759] close(4)                    = 0
    [pid 12760] +++ exited with 0 +++
    --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=12760, si_uid=1000, si_status=0, si_utime=0, si_stime=0} ---
    wait4(12760, [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 12760
    write(1, "zsh exited with status: exit sta"..., 39) = 39
    sigaltstack({ss_sp=NULL, ss_flags=SS_DISABLE, ss_size=8192}, NULL) = 0
    munmap(0x7f1b604bc000, 12288)           = 0
    exit_group(0)                           = ?
    +++ exited with 0 +++

  Here is the end of `ls; exit 0;`:

    [pid 13134] write(1, "Cargo.lock\nCargo.toml\nREADME.md\n"..., 74) = 74
    [pid 13134] close(1 <unfinished ...>
    [pid 13132] <... poll resumed>)         = 1 ([{fd=4, revents=POLLIN}])
    [pid 13134] <... close resumed>)        = 0
    [pid 13132] read(4,  <unfinished ...>
    [pid 13134] close(2 <unfinished ...>
    [pid 13132] <... read resumed>"Cargo.lock\nCargo.toml\nREADME.md\n", 32) = 32
    [pid 13134] <... close resumed>)        = 0
    [pid 13132] read(4,  <unfinished ...>
    [pid 13134] exit_group(0 <unfinished ...>
    [pid 13132] <... read resumed>"src\nstrace-bad.txt\nstrace-good.t", 32) = 32
    [pid 13134] <... exit_group resumed>)   = ?
    [pid 13132] read(4, "xt\ntarget\n", 64) = 10
    [pid 13132] read(4, 0x556f6a2a6f5a, 54) = -1 EAGAIN (Resource temporarily unavailable)
    [pid 13132] poll([{fd=4, events=POLLIN}, {fd=6, events=POLLIN}], 2, -1 <unfinished ...>
    [pid 13134] +++ exited with 0 +++
    [pid 13133] <... rt_sigsuspend resumed>) = ? ERESTARTNOHAND (To be restarted if no handler)
    [pid 13133] --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=13134, si_uid=1000, si_status=0, si_utime=0, si_stime=0} ---
    [pid 13133] rt_sigprocmask(SIG_BLOCK, ~[RTMIN RT_1], [INT CHLD], 8) = 0
    [pid 13133] rt_sigprocmask(SIG_SETMASK, [INT CHLD], ~[KILL STOP RTMIN RT_1], 8) = 0
    [pid 13133] wait4(-1, [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], WNOHANG|WSTOPPED|WCONTINUED, {ru_utime={tv_sec=0, tv_usec=0}, ru_stime={tv_sec=0, tv_usec=1742}, ...}) = 13134
    [pid 13133] kill(-13134, 0)             = -1 ESRCH (No such process)
    [pid 13133] ioctl(10, TIOCSPGRP, [13133]) = 0
    [pid 13133] ioctl(10, TIOCGWINSZ, {ws_row=98, ws_col=340, ws_xpixel=5100, ws_ypixel=2842}) = 0
    [pid 13133] ioctl(10, TCGETS, {c_iflag=ICRNL|IXON|IUTF8, c_oflag=NL0|CR0|TAB0|BS0|VT0|FF0|OPOST|ONLCR, c_cflag=B38400|CS8|CREAD, c_lflag=ISIG|ICANON|ECHO|ECHOE|ECHOK|IEXTEN|ECHOCTL|ECHOKE, ...}) = 0
    [pid 13133] ioctl(10, TIOCGPGRP, [13133]) = 0
    [pid 13133] wait4(-1, 0x7ffff08a27f4, WNOHANG|WSTOPPED|WCONTINUED, 0x7ffff08a2810) = -1 ECHILD (No child processes)
    [pid 13133] rt_sigreturn({mask=[CHLD WINCH]}) = -1 EINTR (Interrupted system call)
    [pid 13133] rt_sigprocmask(SIG_BLOCK, [CHLD], [CHLD WINCH], 8) = 0
    [pid 13133] rt_sigprocmask(SIG_UNBLOCK, [CHLD], [CHLD WINCH], 8) = 0
    [pid 13133] rt_sigprocmask(SIG_BLOCK, [CHLD], [WINCH], 8) = 0
    [pid 13133] rt_sigprocmask(SIG_UNBLOCK, [CHLD], [CHLD WINCH], 8) = 0
    [pid 13133] rt_sigprocmask(SIG_BLOCK, [CHLD], [WINCH], 8) = 0
    [pid 13133] ioctl(10, TIOCSPGRP, [13129]) = 0
    [pid 13133] setpgid(0, 13129)           = 0
    [pid 13133] getpid()                    = 13133
    [pid 13133] exit_group(0)               = ?
    [pid 13132] <... poll resumed>)         = 2 ([{fd=4, revents=POLLHUP}, {fd=6, revents=POLLHUP}])
    [pid 13133] +++ exited with 0 +++
    --- SIGCHLD {si_signo=SIGCHLD, si_code=CLD_EXITED, si_pid=13133, si_uid=1000, si_status=0, si_utime=0, si_stime=0} ---
    read(4, "", 54)                         = 0
    ioctl(6, FIONBIO, [0])                  = 0
    read(6, "", 32)                         = 0
    close(6)                                = 0
    close(4)                                = 0
    wait4(13133, [{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 13133
    write(1, "zsh exited with status: exit sta"..., 39) = 39
    sigaltstack({ss_sp=NULL, ss_flags=SS_DISABLE, ss_size=8192}, NULL) = 0
    munmap(0x7f1defdd4000, 12288)           = 0
    exit_group(0)                           = ?
    +++ exited with 0 +++

## Resources/Maybes/...

- Fish issue on process groups: https://github.com/fish-shell/fish-shell/issues/7060#issuecomment-636421938

- Is it maybe that `/usr/bin/env` and `ls` (both which trigger it) aren't shell built-ins, but `echo` and `exit` etc are?

- Linux man page on process group IDs, session IDs, etc.: https://man7.org/linux/man-pages/man7/credentials.7.html

> A process group (sometimes called a "job") is a collection of processes that share the same process group ID; the shell creates a new process group for the process(es) used to execute single command or pipeline (e.g., the two processes created to execute the command "ls | wc" are placed in the same process group). A process's group membership can be set using setpgid(2). The process whose process ID is the same as its process group ID is the process group leader for that group.

> At most one of the jobs in a session may be the foreground job; other jobs in the session are background jobs.
> Only the foreground job may read from the terminal; when a process in the background attempts to read from the terminal, its process group is sent a SIGTTIN signal, which suspends the job.
> If the TOSTOP flag has been set for the terminal (see termios(3)), then only the foreground job may write to the terminal; writes from background jobs cause a SIGTTOU signal to be generated, which suspends the job.

Here, so what's the foreground job?

> When terminal keys that generate a signal (such as the interrupt key, normally control-C) are pressed, the signal is sent to the processes in the foreground job.

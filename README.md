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

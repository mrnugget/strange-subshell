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
- It happens even if I run `zsh` with `--no-rcs` and/or comment out my `.zprofile`/`.zshrc`/`.profile` completely. I can run ZSH without any config files (`-f` to disable files and `-d` to disable global config files) and it still happens.

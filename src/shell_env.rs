use std::{collections::HashMap, path::Path};

use anyhow::anyhow;
use anyhow::Result;

fn load_shell_environment(dir: &Path) -> Result<HashMap<String, String>> {
    let shell = std::env::var("SHELL")?;

    let command = format!("cd {:?}; /usr/bin/env -0; exit 0;", dir);

    let output = std::process::Command::new(&shell)
        .args(["-i", "-c", &command])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("login shell exited with error {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut parsed_env = HashMap::default();
    for line in stdout.split_terminator('\0') {
        if let Some(separator_index) = line.find('=') {
            let key = line[..separator_index].to_string();
            let value = line[separator_index + 1..].to_string();
            parsed_env.insert(key, value);
        }
    }
    Ok(parsed_env)
}

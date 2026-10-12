use std::process::Command;

pub(crate) fn run(command: &str, args: &[&str]) -> String {
    Command::new(command)
        .args(args)
        .output()
        .map(|output| decode(output.stdout))
        .unwrap_or_default()
}

pub(crate) fn run_checked(command: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        let code = match output.status.code() {
            Some(code) => code.to_string(),
            None => "signal".to_string(),
        };
        let suffix = if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        };
        anyhow::bail!("{command} exited with {code}{suffix}");
    }
    Ok(decode(output.stdout))
}

fn decode(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

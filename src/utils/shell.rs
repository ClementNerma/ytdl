use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

use crate::dl::DEFAULT_BEST_VIDEO_FORMAT;
use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::fail;

fn flush_stdout() {
    io::stdout()
        .flush()
        .unwrap_or_else(|e| fail!("Failed to flush STDOUT: {e}"));

    io::stderr()
        .flush()
        .unwrap_or_else(|e| fail!("Failed to flush STDERR: {e}"));
}

pub fn run_cmd(bin: &Path, args: &[&str]) -> Result<String> {
    run_custom_cmd(Command::new(bin).args(args))
}

pub fn run_custom_cmd(cmd: &mut Command) -> Result<String> {
    flush_stdout();

    let result = cmd.output().context("Failed to run shell command")?;

    flush_stdout();

    ensure_cmd_success(cmd, &result.status, &result.stderr)?;

    let output =
        std::str::from_utf8(&result.stdout).context("Failed to decode command output as UTF-8")?;

    Ok(output.to_string())
}

pub fn run_cmd_bi_outs(
    bin: &Path,
    args: &[&str],
    inspect_err: Option<ShellErrInspector>,
) -> Result<()> {
    run_custom_cmd_bi_outs(Command::new(bin).args(args), inspect_err)
}

pub fn run_custom_cmd_bi_outs(
    cmd: &mut Command,
    inspect_err: Option<ShellErrInspector>,
) -> Result<()> {
    flush_stdout();

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to run shell command")?;

    let mut readbuf = vec![0; 64];
    let mut stderr = child.stderr.take().unwrap();
    let mut stderr_bytes: Vec<u8> = vec![];
    let mut io_stderr = io::stderr();

    let status = loop {
        let size = stderr.read(&mut readbuf)?;
        io_stderr.write_all(&readbuf[0..size])?;
        stderr_bytes.extend(&readbuf[0..size]);

        if let Some(status) = child.try_wait()? {
            break status;
        }
    };

    let stderr = std::str::from_utf8(&stderr_bytes)
        .context("Failed to decode command STDERR output as UTF-8")?;

    ensure_cmd_success(cmd, &status, &stderr_bytes).map_err(|err| {
        if let Some(f) = inspect_err {
            f(stderr);
        }

        err
    })
}

pub fn ensure_cmd_success(cmd: &Command, status: &ExitStatus, stderr: &[u8]) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    let status_code = match status.code() {
        Some(code) => code.to_string(),
        None => String::from("<unknown code>"),
    };

    bail!(
        "Failed to run command (status code = {}).\n\nArguments: {}\n\nSTDERR content:\n\n{}",
        status_code.bright_yellow(),
        cmd.get_args()
            .map(|arg| arg.to_string_lossy())
            // Little hardcoded shortener for a very large and common argument
            .map(|arg| if arg == DEFAULT_BEST_VIDEO_FORMAT {
                "builtin:DEFAULT_BEST_FORMAT".bright_magenta()
            } else {
                arg.bright_yellow()
            })
            .map(|arg| format!("'{}'", arg).bright_cyan().to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .bright_yellow(),
        String::from_utf8_lossy(stderr).bright_yellow()
    );
}

pub type ShellErrInspector<'a> = &'a dyn Fn(&str);

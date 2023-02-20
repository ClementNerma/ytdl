use anyhow::{bail, Context, Result};
use pomsky::{
    options::{CompileOptions, RegexFlavor},
    Expr,
};
use regex::Regex;

pub fn compile_pomsky(input: &str) -> Result<Regex> {
    let (compiled, diag) = Expr::parse_and_compile(
        &format!("{POMSKY_HEADER}{input}"),
        CompileOptions {
            flavor: RegexFlavor::Rust,
            ..Default::default()
        },
    );

    let Some(compiled) = compiled else {
        eprintln!("Warnings/errors while compiled regex '{input}':");

        for diag in diag {
            eprintln!("* {}", diag.msg);
        }

        bail!("Failed to compile regex '{input}'");
    };

    Ok(Regex::new(&compiled)
        .context("Failed to compile the regex")
        .unwrap())
}

static POMSKY_HEADER: &str = "
    let https = \"http\" 's'? \"://\";
    let www = \"www.\"?;
    let id = !['\\']+;
";

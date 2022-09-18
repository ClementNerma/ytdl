use anyhow::Context;
use pomsky::{
    error::CompileError,
    options::{CompileOptions, RegexFlavor},
    Expr,
};
use regex::Regex;

pub fn compile_pomsky(input: &str) -> Result<Regex, CompileError> {
    let (compiled, _) = Expr::parse_and_compile(
        &format!("{POMSKY_HEADER}{input}"),
        CompileOptions {
            flavor: RegexFlavor::Rust,
            ..Default::default()
        },
    )?;

    Ok(Regex::new(&compiled)
        .context("Failed to compile the regex")
        .unwrap())
}

static POMSKY_HEADER: &str = "
    let https = \"http\" 's'? \"://\";
    let www = \"www.\"?;
    let id = !['\\']+;
";

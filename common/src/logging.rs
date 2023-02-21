use anyhow::Result;

pub fn init(level: &str) -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(level)
            .with_line_number(true)
            .with_file(true)
            .json()
            .finish(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests;

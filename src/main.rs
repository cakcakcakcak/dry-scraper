use std::error::Error;

mod config;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pool = config::init_db().await?;

    Ok(())
}

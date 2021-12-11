use config;
use log;
use pretty_env_logger;
use std::path::Path;

mod dgraph;
mod music;
use dgraph::DgraphClient;

/// Default error type
type CanariaError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), CanariaError> {
    pretty_env_logger::init();
    
    let settings = load_config();

    let db_client = DgraphClient::new(settings.get_str("dgraph_url")?);
    if settings.get_bool("set_schema")? || settings.get_bool("drop_all_data")? {
        if settings.get_bool("drop_all_data")? {
            db_client.drop_all().await?;
            log::info!("database dropped")
        } 
        let schemafile = settings.get_str("dgraph_schema")?;
        db_client.set_schema(Path::new(schemafile.as_str())).await?;
        log::info!("database schema set");
    }

    music::library::Library::new(
        settings.get_str("music_library_path")?,
        settings.get_str("music_library_name")?,
        &db_client,
    ).await?;

    Ok(())
}

fn load_config() -> config::Config {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("Syrinx"))
        .unwrap()
        .merge(config::Environment::with_prefix("SYRINX"))
        .unwrap();

    // TODO: check URL for ending slash and remove it
    settings
}
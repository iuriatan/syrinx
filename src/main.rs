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
    let settings = load_config()?;

    let db_client = DgraphClient::new(settings.get_string("dgraph_url")?);
    if settings.get_bool("set_schema")? || settings.get_bool("drop_all_data")? {
        if settings.get_bool("drop_all_data")? {
            db_client.drop_all().await?;
            log::info!("database dropped")
        }
        let schemafile = settings.get_string("dgraph_schema")?;
        db_client.set_schema(Path::new(schemafile.as_str())).await?;
        log::info!("database schema set");
    }

    let mut music_ignore_list = settings.get_array("music_ignore_list").unwrap_or_default();
    let music_ignore_list: Vec<String> = music_ignore_list
        .iter_mut()
        .map(|v| v.clone().into_string().unwrap_or("".into()))
        .collect();
    music::library::Library::new(
        settings.get_string("music_library_path")?,
        settings.get_string("music_library_name")?,
        &db_client,
        music_ignore_list,
    )
    .await?;

    Ok(())
}

fn load_config() -> Result<config::Config, CanariaError> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("Syrinx"))
        .add_source(config::Environment::with_prefix("SYRINX"));
    
    settings.build().map_err(|e| e.into())
}

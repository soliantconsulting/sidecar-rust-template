use anyhow::Context;
use aws_config::BehaviorVersion;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMakerConfig {
    pub hostname: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub script_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub signing_key: String,
    pub file_maker: FileMakerConfig,
}

pub async fn get_config() -> Result<Config, anyhow::Error> {
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let ssm_client = aws_sdk_ssm::Client::new(&aws_config);
    let param = std::env::var("CONFIG_PARAMETER_NAME").context("CONFIG_PARAMETER_NAME not set")?;

    let parameter_result = ssm_client
        .get_parameter()
        .name(&param)
        .with_decryption(true)
        .send()
        .await
        .with_context(|| format!("Failed to load param {}", param))?;

    serde_json::from_str(
        parameter_result
            .parameter
            .context("Missing config parameter")?
            .value
            .context("Missing config parameter value")?
            .as_str(),
    )
    .context("Failed to deserialize config parameter")
}

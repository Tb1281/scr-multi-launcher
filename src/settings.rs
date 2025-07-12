use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "./conf.toml";

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Settings {
    pub path_32: String,
    pub path_64: String,
}

impl Settings {
    pub async fn load() -> Option<Self> {
        use tokio::{fs::File, io::AsyncReadExt};

        let mut file = File::open(CONFIG_PATH).await.ok()?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await.ok()?;

        toml::from_str(&buffer).ok()
    }

    pub async fn save(self) -> Result<Self, String> {
        use tokio::fs::write;

        write(
            "conf.toml",
            toml::to_string_pretty(&self).map_err(|err| err.to_string())?,
        )
        .await
        .map(|_| self)
        .map_err(|err| err.to_string())
    }
}

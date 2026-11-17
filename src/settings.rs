use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("conf.toml")))
        .unwrap_or_else(|| PathBuf::from("conf.toml"))
}

/// 실행 파일 옆의 conf.toml을 우선 사용하고, 없으면 예전 버전처럼
/// 현재 작업 디렉터리의 conf.toml을 사용한다.
async fn load_path() -> PathBuf {
    let exe_path = config_path();
    if tokio::fs::try_exists(&exe_path).await.unwrap_or(false) {
        return exe_path;
    }

    let cwd_path = PathBuf::from("conf.toml");
    if tokio::fs::try_exists(&cwd_path).await.unwrap_or(false) {
        return cwd_path;
    }

    exe_path
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Settings {
    pub path_32: String,
    pub path_64: String,
}

impl Settings {
    pub async fn load() -> Result<Self, String> {
        use tokio::fs::read_to_string;

        let path = load_path().await;
        let buffer = read_to_string(&path)
            .await
            .map_err(|err| format!("{} 파일을 읽을 수 없습니다: {}", path.display(), err))?;

        toml::from_str(&buffer).map_err(|err| {
            format!(
                "{} 파싱에 실패했습니다. Windows 경로의 역슬래시는 \\\\로 쓰거나 작은따옴표('...')로 감싸세요.\n{}",
                path.display(),
                err
            )
        })
    }

    pub async fn save(self) -> Result<Self, String> {
        use tokio::fs::write;

        let path = config_path();
        write(
            &path,
            toml::to_string_pretty(&self).map_err(|err| err.to_string())?,
        )
        .await
        .map(|_| self)
        .map_err(|err| {
            format!(
                "{} 파일을 저장할 수 없습니다: {} (실행 파일이 있는 폴더의 쓰기 권한을 확인하세요)",
                path.display(),
                err
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn missing_path_defaults_instead_of_failing_whole_config() {
        let settings: Settings =
            toml::from_str("path_64 = 'C:\\StarCraft\\x86_64\\StarCraft.exe'").unwrap();

        assert_eq!(settings.path_32, "");
        assert_eq!(settings.path_64, "C:\\StarCraft\\x86_64\\StarCraft.exe");
    }
}

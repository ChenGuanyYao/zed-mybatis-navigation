use std::{env, fs, path::PathBuf};
use zed_extension_api::{self as zed, Architecture, LanguageServerId, Os, Worktree};

// A self-contained macOS Apple Silicon development distribution. No network access is used.
const BUNDLED_MACOS_AARCH64_LSP: &[u8] = include_bytes!("../server/mybatis-lsp-macos-aarch64");

struct MyBatisNavigation;

impl MyBatisNavigation {
    fn binary_path(work_dir: &PathBuf) -> PathBuf {
        work_dir.join("server").join("mybatis-lsp")
    }

    fn install_bundled_binary(work_dir: &PathBuf) -> zed::Result<PathBuf> {
        if !matches!(zed::current_platform(), (Os::Mac, Architecture::Aarch64)) {
            return Err("This self-contained development build bundles a macOS Apple Silicon mybatis-lsp binary. Build a platform-specific package for this system.".to_string());
        }

        let binary = Self::binary_path(work_dir);
        if binary.exists() {
            return Ok(binary);
        }
        fs::create_dir_all(work_dir.join("server")).map_err(|error| error.to_string())?;
        fs::write(&binary, BUNDLED_MACOS_AARCH64_LSP).map_err(|error| error.to_string())?;
        zed::make_file_executable(&binary.to_string_lossy()).map_err(|error| error.to_string())?;
        Ok(binary)
    }
}

impl zed::Extension for MyBatisNavigation {
    fn new() -> Self { Self }

    fn language_server_command(&mut self, language_server_id: &LanguageServerId, _worktree: &Worktree) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() != "mybatis-navigation" {
            return Err(format!("Unknown language server: {language_server_id}"));
        }
        let work_dir = env::current_dir().map_err(|error| format!("Failed to determine extension work directory: {error}"))?;
        let binary = Self::install_bundled_binary(&work_dir)?;
        Ok(zed::Command { command: binary.to_string_lossy().to_string(), args: vec![], env: vec![] })
    }
}

zed::register_extension!(MyBatisNavigation);

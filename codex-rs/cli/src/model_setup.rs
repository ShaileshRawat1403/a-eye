use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use codex_common::oss::get_default_model_for_oss_provider;
use codex_core::LMSTUDIO_OSS_PROVIDER_ID;
use codex_core::OLLAMA_OSS_PROVIDER_ID;
use codex_core::config::edit::ConfigEdit;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_core::config::find_codex_home;
use codex_core::config::set_default_oss_provider;
use std::io;
use std::io::IsTerminal;
use std::io::Write;
use toml_edit::Item as TomlItem;
use toml_edit::value;

/// Manage model providers and guided model setup.
#[derive(Debug, Parser)]
pub struct ModelsCommand {
    #[command(subcommand)]
    pub subcommand: Option<ModelsSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum ModelsSubcommand {
    /// Show built-in preset options for commercial and open-source models.
    List,
    /// Run a guided setup wizard and save provider config.
    Setup(ModelSetupCommand),
}

#[derive(Debug, Clone, Parser, Default)]
pub struct ModelSetupCommand {
    /// Provider preset to configure. If omitted, an interactive picker is shown.
    #[arg(long, value_enum)]
    pub provider: Option<ProviderPreset>,

    /// Optional default model name to save (for example, gpt-5.1-codex or llama3.1:8b).
    #[arg(long)]
    pub model: Option<String>,

    /// Provider base URL. Required for custom/openai-compatible presets when stdin is not interactive.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Environment variable name for API key (for example OPENROUTER_API_KEY).
    #[arg(long = "api-key-env")]
    pub api_key_env: Option<String>,

    /// API version query param (mainly for Azure OpenAI).
    #[arg(long = "api-version")]
    pub api_version: Option<String>,

    /// Custom provider id (used with --provider custom).
    #[arg(long)]
    pub provider_id: Option<String>,

    /// Custom display name (used with --provider custom).
    #[arg(long = "display-name")]
    pub display_name: Option<String>,

    /// Wire API mode for custom/openai-compatible providers.
    #[arg(long, value_enum)]
    pub wire_api: Option<WireApiArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProviderPreset {
    Openai,
    Openrouter,
    Azure,
    AnthropicCompatible,
    GeminiCompatible,
    Ollama,
    Lmstudio,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WireApiArg {
    Responses,
    Chat,
}

impl WireApiArg {
    fn as_config_str(self) -> &'static str {
        match self {
            Self::Responses => "responses",
            Self::Chat => "chat",
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedSetup {
    provider_id: String,
    display_name: String,
    base_url: Option<String>,
    api_key_env: Option<String>,
    wire_api: WireApiArg,
    api_version: Option<String>,
    model: Option<String>,
    oss_provider: Option<&'static str>,
    write_provider_table: bool,
}

pub async fn run(command: ModelsCommand) -> Result<()> {
    match command.subcommand {
        Some(ModelsSubcommand::List) => {
            print_presets();
            Ok(())
        }
        Some(ModelsSubcommand::Setup(setup)) => run_setup(setup).await,
        None => run_setup(ModelSetupCommand::default()).await,
    }
}

pub async fn run_setup_command(cmd: ModelSetupCommand) -> Result<()> {
    run_setup(cmd).await
}

fn print_presets() {
    println!("Model setup presets\n");

    println!("Commercial:");
    println!("  - openai              Built-in OpenAI provider");
    println!("  - openrouter          OpenRouter (OpenAI-compatible)");
    println!("  - azure               Azure OpenAI (OpenAI-compatible)");
    println!("  - anthropic-compatible  Anthropic via OpenAI-compatible gateway");
    println!("  - gemini-compatible     Gemini via OpenAI-compatible gateway");

    println!("\nOpen-source local:");
    println!("  - ollama              Local Ollama");
    println!("  - lmstudio            Local LM Studio");

    println!("\nCustom:");
    println!("  - custom              Any OpenAI-compatible endpoint");

    println!("\nExamples:");
    println!("  a-eye models setup --provider openrouter --model openai/gpt-oss-20b");
    println!("  a-eye models setup --provider ollama --model llama3.1:8b");
    println!("  a-eye models setup --provider custom --provider-id my-gateway \\");
    println!("    --display-name \"My Gateway\" --base-url https://example.com/v1 \\");
    println!("    --api-key-env MY_GATEWAY_API_KEY --wire-api chat");
}

async fn run_setup(cmd: ModelSetupCommand) -> Result<()> {
    let is_interactive = io::stdin().is_terminal() && io::stdout().is_terminal();

    let preset = match cmd.provider {
        Some(preset) => preset,
        None if is_interactive => pick_provider_preset()?,
        None => {
            anyhow::bail!(
                "No provider selected. Re-run with --provider <preset> or run interactively."
            );
        }
    };

    let resolved = resolve_setup(cmd, preset, is_interactive)?;
    persist_setup(&resolved).await?;
    print_summary(&resolved);
    Ok(())
}

fn resolve_setup(
    cmd: ModelSetupCommand,
    preset: ProviderPreset,
    is_interactive: bool,
) -> Result<ResolvedSetup> {
    match preset {
        ProviderPreset::Openai => {
            let model = resolve_model(
                cmd.model,
                None,
                is_interactive,
                "Default model (optional, press Enter to keep current)",
            )?;
            Ok(ResolvedSetup {
                provider_id: "openai".to_string(),
                display_name: "OpenAI".to_string(),
                base_url: None,
                api_key_env: None,
                wire_api: WireApiArg::Responses,
                api_version: None,
                model,
                oss_provider: None,
                write_provider_table: false,
            })
        }
        ProviderPreset::Openrouter => {
            let base_url = resolve_required_string(
                cmd.base_url,
                "https://openrouter.ai/api/v1",
                is_interactive,
                "OpenRouter base URL",
                "--base-url is required for non-interactive setup",
            )?;
            let api_key_env = resolve_required_string(
                cmd.api_key_env,
                "OPENROUTER_API_KEY",
                is_interactive,
                "API key environment variable",
                "--api-key-env is required for non-interactive setup",
            )?;
            Ok(ResolvedSetup {
                provider_id: "openrouter".to_string(),
                display_name: "OpenRouter".to_string(),
                base_url: Some(base_url),
                api_key_env: Some(api_key_env),
                wire_api: cmd.wire_api.unwrap_or(WireApiArg::Chat),
                api_version: None,
                model: cmd.model,
                oss_provider: None,
                write_provider_table: true,
            })
        }
        ProviderPreset::Azure => {
            let base_url = resolve_required_string(
                cmd.base_url,
                "https://YOUR-RESOURCE.openai.azure.com/openai",
                is_interactive,
                "Azure OpenAI base URL",
                "--base-url is required for non-interactive setup",
            )?;
            let api_key_env = resolve_required_string(
                cmd.api_key_env,
                "AZURE_OPENAI_API_KEY",
                is_interactive,
                "API key environment variable",
                "--api-key-env is required for non-interactive setup",
            )?;
            let api_version = resolve_required_string(
                cmd.api_version,
                "2024-10-21",
                is_interactive,
                "Azure API version",
                "--api-version is required for non-interactive setup",
            )?;
            Ok(ResolvedSetup {
                provider_id: "azure".to_string(),
                display_name: "Azure OpenAI".to_string(),
                base_url: Some(base_url),
                api_key_env: Some(api_key_env),
                wire_api: cmd.wire_api.unwrap_or(WireApiArg::Responses),
                api_version: Some(api_version),
                model: cmd.model,
                oss_provider: None,
                write_provider_table: true,
            })
        }
        ProviderPreset::AnthropicCompatible => {
            let base_url = resolve_required_string(
                cmd.base_url,
                "https://YOUR-GATEWAY.example.com/v1",
                is_interactive,
                "Anthropic-compatible gateway base URL",
                "--base-url is required for non-interactive setup",
            )?;
            let api_key_env = resolve_required_string(
                cmd.api_key_env,
                "ANTHROPIC_API_KEY",
                is_interactive,
                "API key environment variable",
                "--api-key-env is required for non-interactive setup",
            )?;
            Ok(ResolvedSetup {
                provider_id: "anthropic-compatible".to_string(),
                display_name: "Anthropic (OpenAI-compatible gateway)".to_string(),
                base_url: Some(base_url),
                api_key_env: Some(api_key_env),
                wire_api: cmd.wire_api.unwrap_or(WireApiArg::Chat),
                api_version: None,
                model: cmd.model,
                oss_provider: None,
                write_provider_table: true,
            })
        }
        ProviderPreset::GeminiCompatible => {
            let base_url = resolve_required_string(
                cmd.base_url,
                "https://YOUR-GATEWAY.example.com/v1",
                is_interactive,
                "Gemini-compatible gateway base URL",
                "--base-url is required for non-interactive setup",
            )?;
            let api_key_env = resolve_required_string(
                cmd.api_key_env,
                "GEMINI_API_KEY",
                is_interactive,
                "API key environment variable",
                "--api-key-env is required for non-interactive setup",
            )?;
            Ok(ResolvedSetup {
                provider_id: "gemini-compatible".to_string(),
                display_name: "Gemini (OpenAI-compatible gateway)".to_string(),
                base_url: Some(base_url),
                api_key_env: Some(api_key_env),
                wire_api: cmd.wire_api.unwrap_or(WireApiArg::Chat),
                api_version: None,
                model: cmd.model,
                oss_provider: None,
                write_provider_table: true,
            })
        }
        ProviderPreset::Ollama => {
            let model = resolve_model(
                cmd.model,
                get_default_model_for_oss_provider(OLLAMA_OSS_PROVIDER_ID),
                is_interactive,
                "Default Ollama model",
            )?;
            Ok(ResolvedSetup {
                provider_id: OLLAMA_OSS_PROVIDER_ID.to_string(),
                display_name: "Ollama".to_string(),
                base_url: None,
                api_key_env: None,
                wire_api: WireApiArg::Responses,
                api_version: None,
                model,
                oss_provider: Some(OLLAMA_OSS_PROVIDER_ID),
                write_provider_table: false,
            })
        }
        ProviderPreset::Lmstudio => {
            let model = resolve_model(
                cmd.model,
                get_default_model_for_oss_provider(LMSTUDIO_OSS_PROVIDER_ID),
                is_interactive,
                "Default LM Studio model",
            )?;
            Ok(ResolvedSetup {
                provider_id: LMSTUDIO_OSS_PROVIDER_ID.to_string(),
                display_name: "LM Studio".to_string(),
                base_url: None,
                api_key_env: None,
                wire_api: WireApiArg::Responses,
                api_version: None,
                model,
                oss_provider: Some(LMSTUDIO_OSS_PROVIDER_ID),
                write_provider_table: false,
            })
        }
        ProviderPreset::Custom => {
            let provider_id = resolve_required_string(
                cmd.provider_id,
                "my-provider",
                is_interactive,
                "Custom provider id",
                "--provider-id is required for non-interactive custom setup",
            )?;
            let display_name = resolve_required_string(
                cmd.display_name,
                "My Provider",
                is_interactive,
                "Custom display name",
                "--display-name is required for non-interactive custom setup",
            )?;
            let base_url = resolve_required_string(
                cmd.base_url,
                "https://example.com/v1",
                is_interactive,
                "Custom base URL",
                "--base-url is required for non-interactive custom setup",
            )?;
            let api_key_env = resolve_required_string(
                cmd.api_key_env,
                "MY_PROVIDER_API_KEY",
                is_interactive,
                "API key environment variable",
                "--api-key-env is required for non-interactive custom setup",
            )?;
            let wire_api = match cmd.wire_api {
                Some(wire_api) => wire_api,
                None if is_interactive => pick_wire_api()?,
                None => {
                    anyhow::bail!(
                        "--wire-api is required for non-interactive custom setup (chat or responses)"
                    );
                }
            };

            Ok(ResolvedSetup {
                provider_id,
                display_name,
                base_url: Some(base_url),
                api_key_env: Some(api_key_env),
                wire_api,
                api_version: None,
                model: cmd.model,
                oss_provider: None,
                write_provider_table: true,
            })
        }
    }
}

fn resolve_model(
    provided: Option<String>,
    default: Option<&str>,
    is_interactive: bool,
    prompt: &str,
) -> Result<Option<String>> {
    if let Some(model) = provided {
        return Ok(Some(model));
    }

    if !is_interactive {
        return Ok(default.map(ToOwned::to_owned));
    }

    let model = prompt_with_default(prompt, default.unwrap_or(""))?;
    if model.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(model))
    }
}

fn resolve_required_string(
    provided: Option<String>,
    default: &str,
    is_interactive: bool,
    prompt: &str,
    non_interactive_error: &str,
) -> Result<String> {
    if let Some(value) = provided
        && !value.trim().is_empty()
    {
        return Ok(value);
    }

    if !is_interactive {
        anyhow::bail!("{non_interactive_error}");
    }

    let value = prompt_with_default(prompt, default)?;
    if value.trim().is_empty() {
        anyhow::bail!("{prompt} cannot be empty");
    }
    Ok(value)
}

async fn persist_setup(resolved: &ResolvedSetup) -> Result<()> {
    let codex_home = find_codex_home()?;

    let mut edits = Vec::new();
    edits.push(set_path(
        &["model_provider"],
        value(resolved.provider_id.clone()),
    ));

    if let Some(model) = resolved.model.as_deref() {
        edits.push(set_path(&["model"], value(model)));
    }

    if resolved.write_provider_table {
        let provider_path = ["model_providers", resolved.provider_id.as_str()];

        edits.push(set_path(
            &[provider_path[0], provider_path[1], "name"],
            value(resolved.display_name.clone()),
        ));

        if let Some(base_url) = resolved.base_url.as_deref() {
            edits.push(set_path(
                &[provider_path[0], provider_path[1], "base_url"],
                value(base_url),
            ));
        }

        if let Some(api_key_env) = resolved.api_key_env.as_deref() {
            edits.push(set_path(
                &[provider_path[0], provider_path[1], "env_key"],
                value(api_key_env),
            ));
        }

        edits.push(set_path(
            &[provider_path[0], provider_path[1], "wire_api"],
            value(resolved.wire_api.as_config_str()),
        ));

        edits.push(set_path(
            &[provider_path[0], provider_path[1], "requires_openai_auth"],
            value(false),
        ));

        if let Some(api_version) = resolved.api_version.as_deref() {
            edits.push(set_path(
                &[
                    provider_path[0],
                    provider_path[1],
                    "query_params",
                    "api-version",
                ],
                value(api_version),
            ));
        }
    }

    ConfigEditsBuilder::new(&codex_home)
        .with_edits(edits)
        .apply()
        .await
        .context("failed to persist model provider setup")?;

    if let Some(oss_provider) = resolved.oss_provider {
        set_default_oss_provider(&codex_home, oss_provider)
            .with_context(|| format!("failed to set default OSS provider to {oss_provider}"))?;
    }

    Ok(())
}

fn set_path(path: &[&str], value: TomlItem) -> ConfigEdit {
    ConfigEdit::SetPath {
        segments: path.iter().map(|segment| (*segment).to_string()).collect(),
        value,
    }
}

fn print_summary(resolved: &ResolvedSetup) {
    let wire_api = resolved.wire_api.as_config_str();
    println!("\nModel setup saved.");
    println!(
        "  Provider ID: {provider_id}",
        provider_id = resolved.provider_id
    );
    println!(
        "  Display Name: {display_name}",
        display_name = resolved.display_name
    );
    println!("  Wire API: {wire_api}");

    if let Some(base_url) = resolved.base_url.as_deref() {
        println!("  Base URL: {base_url}");
    }

    if let Some(api_key_env) = resolved.api_key_env.as_deref() {
        println!("\nNext step:");
        println!("  export {api_key_env}=<your_api_key>");
    }

    if let Some(model) = resolved.model.as_deref() {
        println!("  Default model: {model}");
    }

    println!("\nRun `a-eye` to start the interactive UI with this provider.");
}

fn pick_provider_preset() -> Result<ProviderPreset> {
    println!("Select a model provider preset:");
    println!("  1) OpenAI (commercial)");
    println!("  2) OpenRouter (commercial)");
    println!("  3) Azure OpenAI (commercial)");
    println!("  4) Anthropic via OpenAI-compatible gateway");
    println!("  5) Gemini via OpenAI-compatible gateway");
    println!("  6) Ollama (open-source, local)");
    println!("  7) LM Studio (open-source, local)");
    println!("  8) Custom OpenAI-compatible endpoint");

    loop {
        let choice = prompt("Enter choice [1-8]")?;
        let preset = match choice.trim() {
            "1" => Some(ProviderPreset::Openai),
            "2" => Some(ProviderPreset::Openrouter),
            "3" => Some(ProviderPreset::Azure),
            "4" => Some(ProviderPreset::AnthropicCompatible),
            "5" => Some(ProviderPreset::GeminiCompatible),
            "6" => Some(ProviderPreset::Ollama),
            "7" => Some(ProviderPreset::Lmstudio),
            "8" => Some(ProviderPreset::Custom),
            _ => None,
        };

        if let Some(preset) = preset {
            return Ok(preset);
        }

        println!("Invalid choice. Enter a number from 1 to 8.");
    }
}

fn pick_wire_api() -> Result<WireApiArg> {
    println!("Select wire API mode:");
    println!("  1) responses");
    println!("  2) chat");

    loop {
        let choice = prompt("Enter choice [1-2]")?;
        let wire_api = match choice.trim() {
            "1" => Some(WireApiArg::Responses),
            "2" => Some(WireApiArg::Chat),
            _ => None,
        };

        if let Some(wire_api) = wire_api {
            return Ok(wire_api);
        }

        println!("Invalid choice. Enter 1 or 2.");
    }
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_with_default(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn wire_api_to_config_value() {
        assert_eq!(WireApiArg::Responses.as_config_str(), "responses");
        assert_eq!(WireApiArg::Chat.as_config_str(), "chat");
    }

    #[test]
    fn set_path_builds_segments() {
        let edit = set_path(
            &["model_providers", "openrouter", "wire_api"],
            value("chat"),
        );
        let ConfigEdit::SetPath { segments, .. } = edit else {
            panic!("expected set path edit");
        };
        assert_eq!(
            segments,
            vec![
                "model_providers".to_string(),
                "openrouter".to_string(),
                "wire_api".to_string()
            ]
        );
    }
}

use std::path::PathBuf;
use std::process::ExitCode;
use std::{fs, io};

use clap::{Args as ClapArgs, Parser, Subcommand};
use focal_mcp::{BackendConfig, OpenMode, ServerConfig, TransportConfig};

#[derive(Debug, Parser)]
#[command(name = "focal-mcp")]
#[command(about = "Run a Focal MCP stdio server from a YAML config file")]
struct Cli {
    #[arg(long, value_name = "PATH", global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Walk through and write a Focal MCP YAML config file.
    Init(InitArgs),
}

#[derive(Debug, ClapArgs)]
struct InitArgs {
    /// Overwrite an existing config file without asking.
    #[arg(long)]
    force: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(Command::Init(args)) => init_config(cli.config, args.force),
        None => run(cli.config),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(config_path: Option<PathBuf>) -> Result<(), focal_mcp::Error> {
    let config_path = match config_path {
        Some(path) => path,
        None => focal_mcp::default_config_path()?,
    };
    let config = focal_mcp::load_config(config_path)?;
    focal_mcp::run_stdio(config)
}

fn init_config(config_path: Option<PathBuf>, force: bool) -> Result<(), focal_mcp::Error> {
    let mut input = io::BufReader::new(io::stdin());
    let mut output = io::stdout();
    let _path = init_config_with_io(config_path, force, &mut input, &mut output)?;
    Ok(())
}

fn init_config_with_io<R, W>(
    config_path: Option<PathBuf>,
    force: bool,
    input: &mut R,
    output: &mut W,
) -> Result<PathBuf, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    let config_path = match config_path {
        Some(path) => path,
        None => focal_mcp::default_config_path()?,
    };

    writeln!(output, "Focal MCP config setup").map_err(prompt_error)?;
    writeln!(output, "Config path: {}", config_path.display()).map_err(prompt_error)?;

    if fs::symlink_metadata(&config_path).is_ok()
        && !force
        && !prompt_bool(input, output, "Config already exists. Overwrite it?", false)?
    {
        return Err(focal_mcp::Error::InvalidConfig(
            "config initialization cancelled".to_string(),
        ));
    }

    #[cfg(feature = "sqlite")]
    let backend_choices = ["fs", "sqlite"];
    #[cfg(not(feature = "sqlite"))]
    let backend_choices = ["fs"];

    let backend = prompt_choice(input, output, "Backend", &backend_choices, "fs")?;
    let mode = prompt_open_mode(input, output)?;
    let backend = match backend.as_str() {
        "fs" => {
            let root = prompt_path(
                input,
                output,
                "Filesystem graph root",
                default_graph_root()?,
            )?;
            BackendConfig::Fs { root, mode }
        }
        #[cfg(feature = "sqlite")]
        "sqlite" => {
            let database_path = prompt_path(
                input,
                output,
                "SQLite database path",
                default_home_path(".local/share/focal/focal.db")
                    .unwrap_or_else(|| PathBuf::from("focal.db")),
            )?;
            let graph_name = prompt_string(input, output, "SQLite graph name", "default")?;
            BackendConfig::Sqlite {
                database_path,
                graph_name,
                mode,
            }
        }
        _ => {
            return Err(focal_mcp::Error::InvalidConfig(
                "unsupported backend selection".to_string(),
            ));
        }
    };

    let config = ServerConfig {
        backend,
        transport: TransportConfig::Stdio,
    };
    focal_mcp::save_config(&config_path, &config)?;
    writeln!(
        output,
        "Wrote config. Run `focal-mcp` to start the stdio server."
    )
    .map_err(prompt_error)?;
    Ok(config_path)
}

fn prompt_open_mode<R, W>(input: &mut R, output: &mut W) -> Result<OpenMode, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    match prompt_choice(input, output, "Open mode", &["init", "open"], "init")?.as_str() {
        "init" => Ok(OpenMode::Init),
        "open" => Ok(OpenMode::Open),
        _ => Err(focal_mcp::Error::InvalidConfig(
            "unsupported open mode selection".to_string(),
        )),
    }
}

fn prompt_path<R, W>(
    input: &mut R,
    output: &mut W,
    label: &str,
    default: PathBuf,
) -> Result<PathBuf, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    let default = default.to_string_lossy().into_owned();
    prompt_string(input, output, label, &default).map(expand_tilde)
}

fn prompt_choice<R, W>(
    input: &mut R,
    output: &mut W,
    label: &str,
    choices: &[&str],
    default: &str,
) -> Result<String, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    loop {
        let answer = prompt_string(
            input,
            output,
            &format!("{label} ({})", choices.join("/")),
            default,
        )?
        .to_ascii_lowercase();
        if choices.iter().any(|choice| *choice == answer) {
            return Ok(answer);
        }
        writeln!(output, "Please enter one of: {}", choices.join(", ")).map_err(prompt_error)?;
    }
}

fn prompt_bool<R, W>(
    input: &mut R,
    output: &mut W,
    label: &str,
    default: bool,
) -> Result<bool, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    let default_label = if default { "Y/n" } else { "y/N" };
    loop {
        let answer = prompt_string(input, output, &format!("{label} [{default_label}]"), "")?;
        if answer.is_empty() {
            return Ok(default);
        }
        match answer.to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(output, "Please enter yes or no.").map_err(prompt_error)?,
        }
    }
}

fn prompt_string<R, W>(
    input: &mut R,
    output: &mut W,
    label: &str,
    default: &str,
) -> Result<String, focal_mcp::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    if default.is_empty() {
        write!(output, "{label}: ").map_err(prompt_error)?;
    } else {
        write!(output, "{label} [{default}]: ").map_err(prompt_error)?;
    }
    output.flush().map_err(prompt_error)?;

    let mut line = String::new();
    input.read_line(&mut line).map_err(prompt_error)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn default_home_path(relative: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(relative))
}

fn default_graph_root() -> Result<PathBuf, focal_mcp::Error> {
    std::env::current_dir()
        .map(|cwd| cwd.join("spec"))
        .map_err(|source| focal_mcp::Error::Io {
            context: "failed to resolve default filesystem graph root".to_string(),
            source,
        })
}

fn expand_tilde(value: String) -> PathBuf {
    if value == "~" {
        return default_home_path("").unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return default_home_path(rest).unwrap_or_else(|| PathBuf::from(value));
    }
    PathBuf::from(value)
}

fn prompt_error(source: io::Error) -> focal_mcp::Error {
    focal_mcp::Error::Io {
        context: "failed during interactive config setup".to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn init_config_writes_filesystem_config_from_prompts() {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("config/focal.yaml");
        let graph_root = tempdir.path().join("ideas");
        let input = format!("fs\nopen\n{}\n", graph_root.display());
        let mut input = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let written =
            init_config_with_io(Some(config_path.clone()), false, &mut input, &mut output)
                .expect("init config");
        let loaded = focal_mcp::load_config(&config_path).expect("load config");

        assert_eq!(written, config_path);
        assert_eq!(
            loaded,
            ServerConfig {
                backend: BackendConfig::Fs {
                    root: graph_root,
                    mode: OpenMode::Open,
                },
                transport: TransportConfig::Stdio,
            }
        );
    }

    #[test]
    fn init_config_defaults_filesystem_graph_root_to_cwd_spec() {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("config/focal.yaml");
        let graph_root = default_graph_root().expect("default graph root");
        let mut input = Cursor::new(b"fs\ninit\n\n".to_vec());
        let mut output = Vec::new();

        init_config_with_io(Some(config_path.clone()), false, &mut input, &mut output)
            .expect("init config");
        let loaded = focal_mcp::load_config(&config_path).expect("load config");

        assert_eq!(
            loaded,
            ServerConfig {
                backend: BackendConfig::Fs {
                    root: graph_root,
                    mode: OpenMode::Init,
                },
                transport: TransportConfig::Stdio,
            }
        );
    }

    #[test]
    fn init_config_respects_existing_config_without_force() {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("config.yaml");
        fs::write(&config_path, "existing").expect("write existing config");
        let mut input = Cursor::new(b"n\n".to_vec());
        let mut output = Vec::new();

        let error = init_config_with_io(Some(config_path.clone()), false, &mut input, &mut output)
            .expect_err("declined overwrite should fail");
        let contents = fs::read_to_string(config_path).expect("read existing config");

        assert!(error.to_string().contains("cancelled"));
        assert_eq!(contents, "existing");
    }

    #[test]
    fn expand_tilde_leaves_regular_paths_unchanged() {
        assert_eq!(
            expand_tilde("/tmp/focal".to_string()),
            PathBuf::from("/tmp/focal")
        );
    }
}

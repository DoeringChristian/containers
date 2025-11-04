use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use users::{get_current_gid, get_current_uid};

mod config;
mod dockerfile;
mod lockfile;

use config::{ContainerConfig, ContainersToml};
use dockerfile::DockerfileGenerator;
use lockfile::Lockfile;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    docker_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        #[arg(short, long, default_value = "default")]
        container: String,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    Build {
        #[arg(short, long)]
        container: Option<String>,
    },
    Init,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Init) => init_config(),
        Some(Commands::Build { container }) => build_containers(container),
        Some(Commands::Run { container, command }) => run_container(&container, command),
        None => run_legacy_mode(args.docker_args),
    }
}

fn init_config() -> Result<()> {
    let config_path = Path::new("containers.toml");

    if config_path.exists() {
        anyhow::bail!("containers.toml already exists");
    }

    let mut containers = std::collections::HashMap::new();

    let mut default_config = ContainerConfig::default();
    default_config.name = "default".to_string();
    default_config.base_image = Some("ubuntu:latest".to_string());
    default_config.gpu = Some(true);

    containers.insert("default".to_string(), default_config);

    let toml_config = ContainersToml { containers };
    toml_config.save(config_path)?;

    println!("Created containers.toml");
    Ok(())
}

fn build_containers(container: Option<String>) -> Result<()> {
    let config_path = Path::new("containers.toml");
    let config = ContainersToml::from_file(config_path)?;

    let lockfile = Lockfile::generate_from_config(&config.containers)?;
    lockfile.save("containers.lock")?;

    let containers_to_build: Vec<_> = if let Some(name) = container {
        vec![name]
    } else {
        config.containers.keys().cloned().collect()
    };

    for container_name in containers_to_build {
        let container_config = config
            .containers
            .get(&container_name)
            .with_context(|| format!("Container '{}' not found in config", container_name))?;

        let container_lock = lockfile
            .containers
            .get(&container_name)
            .with_context(|| format!("Container '{}' not found in lockfile", container_name))?;

        let dockerfile_content = DockerfileGenerator::generate(container_config, container_lock);

        let dockerfile_dir = Path::new("dockerfiles");
        fs::create_dir_all(dockerfile_dir)?;

        let dockerfile_path = dockerfile_dir.join(format!("Dockerfile.{}", container_name));
        DockerfileGenerator::save(&dockerfile_content, &dockerfile_path)?;

        let entrypoint_content = DockerfileGenerator::generate_entrypoint();
        let entrypoint_path = dockerfile_dir.join("entrypoint.sh");
        fs::write(&entrypoint_path, entrypoint_content)?;

        println!("Building container '{}'...", container_name);

        let mut build_cmd = Command::new("docker");
        build_cmd.args([
            "build",
            "-t",
            &container_lock.image_hash,
            "-f",
            dockerfile_path.to_str().unwrap(),
            dockerfile_dir.to_str().unwrap(),
        ]);

        let status = build_cmd.status()?;

        if !status.success() {
            anyhow::bail!("Failed to build container '{}'", container_name);
        }

        println!("Successfully built container '{}'", container_name);
    }

    Ok(())
}

fn run_container(container_name: &str, command: Vec<String>) -> Result<()> {
    let config_path = Path::new("containers.toml");
    let config = ContainersToml::from_file(config_path)?;

    let container_config = config
        .get_container(container_name)
        .with_context(|| format!("Container '{}' not found", container_name))?;

    let lockfile_path = Path::new("containers.lock");
    let lockfile = if lockfile_path.exists() {
        Lockfile::from_file(lockfile_path)?
    } else {
        anyhow::bail!("No lockfile found. Run 'containers build' first");
    };

    let container_lock = lockfile
        .containers
        .get(container_name)
        .with_context(|| format!("Container '{}' not found in lockfile", container_name))?;

    let uid = get_current_uid();
    let gid = get_current_gid();
    let current_dir = env::current_dir()?;
    let home_dir = dirs::home_dir().context("Failed to get home directory")?;

    let mut docker_cmd = Command::new("docker");
    docker_cmd.arg("run");

    if container_config.remove.unwrap_or(true) {
        docker_cmd.arg("--rm");
    }

    if container_config.interactive.unwrap_or(true) {
        docker_cmd.arg("-i");
    }

    if container_config.tty.unwrap_or(true) {
        docker_cmd.arg("-t");
    }

    docker_cmd.args(["-e", &format!("UID={}", uid)]);
    docker_cmd.args(["-e", &format!("GID={}", gid)]);

    if let Some(env_vars) = &container_config.environment {
        for (key, value) in env_vars {
            docker_cmd.args(["-e", &format!("{}={}", key, value)]);
        }
    }

    if let Some(volumes) = &container_config.volumes {
        for volume in volumes {
            let mount_str = if volume.read_only.unwrap_or(false) {
                format!("{}:{}:ro", volume.source, volume.target)
            } else {
                format!("{}:{}", volume.source, volume.target)
            };
            docker_cmd.args(["-v", &mount_str]);
        }
    } else {
        docker_cmd.args(["-v", &format!("{}:/home/code/work", current_dir.display())]);
        docker_cmd.args([
            "-v",
            &format!("{}/.claude:/home/code/.claude", home_dir.display()),
        ]);
        docker_cmd.args([
            "-v",
            &format!(
                "{}/.claude.json:/home/code/.claude.json",
                home_dir.display()
            ),
        ]);
    }

    if let Some(tmpfs_mounts) = &container_config.tmpfs {
        for tmpfs in tmpfs_mounts {
            let mut tmpfs_str = tmpfs.target.clone();
            let mut opts = Vec::new();

            if tmpfs.read_only.unwrap_or(false) {
                opts.push("ro".to_string());
            }

            if let Some(size) = &tmpfs.size {
                opts.push(format!("size={}", size));
            }

            if !opts.is_empty() {
                tmpfs_str.push(':');
                tmpfs_str.push_str(&opts.join(","));
            }

            docker_cmd.args(["--tmpfs", &tmpfs_str]);
        }
    } else {
        docker_cmd.args(["--tmpfs", "/home/code/work/build:ro,size=1m"]);
    }

    if container_config.gpu.unwrap_or(false) {
        docker_cmd.args(["--gpus", "all"]);
    }

    docker_cmd.arg(&container_lock.image_hash);

    let final_command = if !command.is_empty() {
        command
    } else if let Some(default_cmd) = &container_config.command {
        default_cmd.clone()
    } else {
        vec![]
    };

    for arg in final_command {
        docker_cmd.arg(arg);
    }

    let status = docker_cmd.spawn()?.wait()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn run_legacy_mode(command: Vec<String>) -> Result<()> {
    let uid = get_current_uid();
    let gid = get_current_gid();
    let current_dir = env::current_dir()?;
    let home_dir = dirs::home_dir().context("Failed to get home directory")?;

    let command = if command.is_empty() {
        vec![
            "claude".to_string(),
            "--dangerously-skip-permissions".to_string(),
            "--max-turns".to_string(),
            "99999999".to_string(),
        ]
    } else {
        command
    };

    let mut docker_cmd = Command::new("docker");

    docker_cmd.args([
        "run",
        "--rm",
        "-it",
        "-e",
        &format!("UID={}", uid),
        "-e",
        &format!("GID={}", gid),
        "-v",
        &format!("{}:/home/code/work", current_dir.display()),
        "--tmpfs",
        "/home/code/work/build:ro,size=1m",
        "-v",
        &format!("{}/.claude:/home/code/.claude", home_dir.display()),
        "-v",
        &format!(
            "{}/.claude.json:/home/code/.claude.json",
            home_dir.display()
        ),
        "-t",
        "--gpus",
        "all",
        "claude",
    ]);

    for arg in command {
        docker_cmd.arg(arg);
    }

    let status = docker_cmd.spawn()?.wait()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}


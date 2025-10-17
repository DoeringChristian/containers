use clap::Parser;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

#[derive(Parser)]
#[command(
    name = "containers",
    about = "Create or enter a container environment",
    after_help = "ENVIRONMENT VARIABLES:
  CONTAINER_NAME          Set default container name
  DOCKERFILE              Set default Dockerfile path
  CONTAINER_ENGINE        Container engine to use (default: podman)

EXAMPLES:
  containers                      Use default settings
  containers mycontainer          Use custom container name
  containers -f custom.dockerfile Use custom Dockerfile
  containers -u                   Update/rebuild image and container
  CONTAINER_ENGINE=docker containers    Use Docker instead of Podman"
)]
struct Args {
    /// Use specified Dockerfile (default: search current dir upward)
    #[arg(short, long, value_name = "PATH")]
    dockerfile: Option<PathBuf>,

    /// Rebuild image and recreate container
    #[arg(short, long)]
    update: bool,

    /// Name for the container (default: based on Dockerfile directory)
    #[arg(value_name = "CONTAINER_NAME")]
    container_name: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let update_image = args.update;
    let container_engine = env::var("CONTAINER_ENGINE").unwrap_or_else(|_| "podman".to_string());

    // Find Dockerfile
    let dockerfile = if let Some(dockerfile) = args.dockerfile {
        dockerfile
    } else if let Ok(dockerfile) = env::var("DOCKERFILE") {
        PathBuf::from(dockerfile)
    } else {
        find_dockerfile().unwrap_or_else(|| {
            let exe_path = env::current_exe().unwrap_or_default();
            let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
            exe_dir.join("Dockerfile")
        })
    };

    // Set container name
    let default_container_name = generate_container_name(&dockerfile);
    let container_name = if let Some(name) = args.container_name {
        name
    } else {
        env::var("CONTAINER_NAME").unwrap_or(default_container_name)
    };

    let image_name = "dev-env:latest";

    // Detect NVIDIA GPU support
    let nvidia_args = detect_nvidia_support(&container_engine);

    // Build image if needed
    if dockerfile.exists() {
        let should_build = update_image || !image_exists(&container_engine, image_name)?;

        if should_build {
            if update_image {
                println!("Updating image: {}", image_name);
                // Remove existing container if it exists
                if container_exists(&container_engine, &container_name)? {
                    println!("Removing existing container: {}", container_name);
                    remove_container(&container_engine, &container_name)?;
                }
            } else {
                println!("Building image: {}", image_name);
            }

            build_image(&container_engine, image_name, &dockerfile)?;
        }
    }

    // Handle container lifecycle
    if container_exists(&container_engine, &container_name)? {
        if container_running(&container_engine, &container_name)? {
            println!("Entering running container: {}", container_name);
            exec_container(&container_engine, &container_name)?;
        } else {
            println!("Starting existing container: {}", container_name);
            start_container(&container_engine, &container_name)?;
            exec_container(&container_engine, &container_name)?;
        }
    } else {
        println!("Creating new container: {}", container_name);
        let current_dir = env::current_dir()?;
        create_and_run_container(
            &container_engine,
            &container_name,
            image_name,
            &current_dir,
            &nvidia_args,
        )?;
    }

    Ok(())
}

fn find_dockerfile() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    let home_dir = home::home_dir()?;

    loop {
        let dockerfile = dir.join("Dockerfile");
        if dockerfile.exists() {
            return Some(dockerfile);
        }

        if dir == home_dir {
            break;
        }

        if dir == Path::new("/") {
            break;
        }

        dir = dir.parent()?.to_path_buf();
    }

    // Check home directory
    let home_dockerfile = home_dir.join("Dockerfile");
    if home_dockerfile.exists() {
        return Some(home_dockerfile);
    }

    None
}

fn generate_container_name(dockerfile: &Path) -> String {
    let dir = dockerfile.parent().unwrap_or_else(|| Path::new("."));
    let path_str = dir.to_string_lossy();

    // Remove leading slash and replace slashes with dashes
    path_str
        .strip_prefix('/')
        .unwrap_or(&path_str)
        .replace('/', "-")
}

fn detect_nvidia_support(container_engine: &str) -> Vec<String> {
    let mut args = Vec::new();

    // Check if nvidia-smi exists and works
    if which::which("nvidia-smi").is_ok() {
        if let Ok(status) = ProcessCommand::new("nvidia-smi")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                match container_engine {
                    "docker" => {
                        args.push("--gpus".to_string());
                        args.push("all".to_string());
                    }
                    "podman" => {
                        args.push("--device".to_string());
                        args.push("nvidia.com/gpu=all".to_string());
                        args.push("--security-opt".to_string());
                        args.push("label=disable".to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    args
}

fn image_exists(
    container_engine: &str,
    image_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let output = ProcessCommand::new(container_engine)
        .arg("images")
        .arg("--format")
        .arg("table {{.Repository}}:{{.Tag}}")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(output_str.lines().any(|line| {
        line.ends_with(image_name) || line.ends_with(&format!("localhost/{}", image_name))
    }))
}

fn container_exists(
    container_engine: &str,
    container_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let output = ProcessCommand::new(container_engine)
        .arg("ps")
        .arg("-a")
        .arg("--format")
        .arg("table {{.Names}}")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(output_str.lines().any(|line| line == container_name))
}

fn container_running(
    container_engine: &str,
    container_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let output = ProcessCommand::new(container_engine)
        .arg("ps")
        .arg("--format")
        .arg("table {{.Names}}")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(output_str.lines().any(|line| line == container_name))
}

fn remove_container(
    container_engine: &str,
    container_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ProcessCommand::new(container_engine)
        .arg("rm")
        .arg("-f")
        .arg(container_name)
        .status()?;
    Ok(())
}

fn build_image(
    container_engine: &str,
    image_name: &str,
    dockerfile: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = ProcessCommand::new(container_engine)
        .arg("build")
        .arg("-t")
        .arg(image_name)
        .arg("-f")
        .arg(dockerfile)
        .arg(".")
        .status()?;

    if !status.success() {
        return Err("Failed to build image".into());
    }
    Ok(())
}

fn start_container(
    container_engine: &str,
    container_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ProcessCommand::new(container_engine)
        .arg("start")
        .arg(container_name)
        .status()?;
    Ok(())
}

fn exec_container(
    container_engine: &str,
    container_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = ProcessCommand::new(container_engine)
        .arg("exec")
        .arg("-it")
        .arg(container_name)
        .arg("/bin/bash")
        .status()?;

    if !status.success() {
        return Err("Failed to exec into container".into());
    }
    Ok(())
}

fn create_and_run_container(
    container_engine: &str,
    container_name: &str,
    image_name: &str,
    current_dir: &Path,
    nvidia_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = ProcessCommand::new(container_engine);
    cmd.arg("run")
        .arg("-it")
        .arg("--name")
        .arg(container_name)
        .arg("-v")
        .arg(format!(
            "{}:{}",
            current_dir.display(),
            current_dir.display()
        ))
        .arg("-w")
        .arg(current_dir);

    // Add NVIDIA arguments
    for arg in nvidia_args {
        cmd.arg(arg);
    }

    cmd.arg(image_name).arg("/bin/bash");

    let status = cmd.status()?;

    if !status.success() {
        return Err("Failed to create and run container".into());
    }
    Ok(())
}


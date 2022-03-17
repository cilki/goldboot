use crate::{config::Config, profiles, qemu::generate_qemuargs};
use simple_error::bail;
use std::{env, error::Error, fs, path::Path};

/// Choose some arbitrary disk and get its size in bytes. The user will likely change it
/// in the config later.
fn guess_disk_size() -> u64 {
    if cfg!(target_os = "unix") {
        // TODO
    }
    return 64000000000;
}

/// Choose some arbitrary memory size in megabytes which is less than the amount of available free memory on the system.
fn guess_memory_size() -> u64 {
    return 2048;
}

pub fn init(
    profiles: &Vec<String>,
    template: &Option<String>,
    name: &Option<String>,
    memory: &Option<String>,
    disk: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    let config_path = Path::new("goldboot.json");

    if config_path.exists() {
        bail!("This directory has already been initialized. Delete goldboot.json to reinitialize.");
    }

    // Create a new config to be filled in according to the given arguments
    let mut config = Config::default();

    // Set name equal to directory name
    if let Some(name) = env::current_dir()?.file_name() {
        config.name = name.to_str().unwrap().to_string();
    }

    // Setup the config for the given base profile
    if profiles.len() > 0 {
        // Generate QEMU flags for this hardware
        config.qemuargs = generate_qemuargs()?;

        // Set current platform
        config.arch = if cfg!(target_arch = "x86_64") {
            Some("x86_64".into())
        } else if cfg!(target_arch = "aarch64") {
            Some("aarch64".into())
        } else {
            panic!("Unsupported platform");
        };

        // Set an arbitrary disk size unless given a value
        config.disk_size = if let Some(disk_size) = disk {
            disk_size.to_string()
        } else {
            format!("{}b", guess_disk_size())
        };

        // Set an arbitrary memory size unless given a value
        config.memory = if let Some(memory_size) = memory {
            memory_size.to_string()
        } else {
            format!("{}", guess_memory_size())
        };

        // Run profile-specific initialization
        for profile in profiles {
            match profile.as_str() {
                "ArchLinux" => {
                    config.ArchLinux = Some(profiles::arch_linux::ArchLinuxProfile::default())
                }
                "Windows10" => {
                    config.Windows10 = Some(profiles::windows_10::Windows10Profile::default())
                }
                "UbuntuServer" => {
                    config.UbuntuServer =
                        Some(profiles::ubuntu_server::UbuntuServerProfile::default())
                }
                "PopOs" => config.PopOs = Some(profiles::pop_os::PopOsProfile::default()),
                _ => panic!("Unknown profile"),
            }
        }
    }
    // Setup the config for the given packer template
    else if let Some(template_value) = template {
        config.packer_template = Some(template_value.to_owned());
    }

    // Finally write out the config
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}
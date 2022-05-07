use goldboot_core::{templates::TemplateType, *};
use simple_error::bail;
use std::{env, error::Error, fs, path::Path};

/// Choose some arbitrary memory size in megabytes which is less than the amount of available free
/// memory on the system.
fn guess_memory_size() -> u64 {
	return 2048;
}

pub fn init(
	templates: &Vec<String>,
	name: &Option<String>,
	memory: &Option<String>,
	disk: &Option<String>,
) -> Result<(), Box<dyn Error>> {
	let config_path = Path::new("goldboot.json");

	if config_path.exists() {
		bail!("This directory has already been initialized. Delete goldboot.json to reinitialize.");
	}

	if templates.len() == 0 {
		bail!("Specify at least one template with --template");
	}

	// Create a new config to be filled in according to the given arguments
	let mut config = BuildConfig::default();

	if let Some(name) = name {
		config.name = name.to_string();
	} else {
		// Set name equal to directory name
		if let Some(name) = env::current_dir()?.file_name() {
			config.name = name.to_str().unwrap().to_string();
		}
	}

	// Generate QEMU flags for this hardware
	//config.qemuargs = generate_qemuargs()?;

	// Set current platform
	config.arch = if cfg!(target_arch = "x86_64") {
		Some("x86_64".into())
	} else if cfg!(target_arch = "aarch64") {
		Some("aarch64".into())
	} else {
		bail!("Unsupported platform");
	};

	// Set an arbitrary memory size unless given a value
	config.memory = if let Some(memory_size) = memory {
		memory_size.to_string()
	} else {
		format!("{}", guess_memory_size())
	};

	// Run template-specific initialization
	let mut default_templates = Vec::new();
	for template in templates {
		let t: TemplateType =
			serde_json::from_str(format!("{{\"type\": \"{}\"}}", &template).as_str())?;
		default_templates.push(t.get_default_template()?);
	}
	config.templates = default_templates;

	// Finally write out the config
	fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
	Ok(())
}
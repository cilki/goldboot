#![feature(derive_default_enum)]

use crate::{
	ssh::SshConnection,
	templates::{Template, TemplateType},
};
use goldboot_image::GoldbootImage;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use simple_error::bail;
use std::{default::Default, error::Error, fs::File, path::Path, process::Command};
use validator::Validate;

pub mod build;
pub mod cache;
pub mod image;
pub mod qemu;
pub mod ssh;
pub mod templates;
pub mod vnc;
pub mod windows;

#[derive(rust_embed::RustEmbed)]
#[folder = "res/"]
struct Resources;

/// Search filesystem for UEFI firmware.
pub fn find_ovmf() -> Option<String> {
	for path in vec![
		"/usr/share/ovmf/x64/OVMF.fd",
		"/usr/share/OVMF/OVMF_CODE.fd",
	] {
		if Path::new(&path).is_file() {
			debug!("Located OVMF firmware at: {}", path.to_string());
			return Some(path.to_string());
		}
	}

	debug!("Failed to locate existing OVMF firmware");
	None
}

/// The global configuration
#[derive(Clone, Serialize, Deserialize, Validate, Default, Debug)]
pub struct BuildConfig {
	/// The image name
	#[validate(length(min = 1))]
	pub name: String,

	/// An image description
	#[validate(length(max = 4096))]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub arch: Option<String>,

	/// The amount of memory to allocate to the VM
	pub memory: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub nvme: Option<bool>,

	#[validate(length(min = 1))]
	pub templates: Vec<serde_json::Value>,
}

impl BuildConfig {
	/// Read the config from an existing image's metadata.
	pub fn from_image(image: &GoldbootImage) -> Result<Self, Box<dyn Error>> {
		let config: BuildConfig = serde_json::from_slice(&image.header.metadata)?;
		Ok(config)
	}

	pub fn get_templates(&self) -> Result<Vec<Box<dyn Template>>, Box<dyn Error>> {
		let mut templates: Vec<Box<dyn Template>> = Vec::new();

		for template in &self.templates {
			// Get type
			let t: TemplateType = serde_json::from_value(template.to_owned())?;
			templates.push(t.parse_template(template.to_owned())?);
		}

		Ok(templates)
	}
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct Partition {
	pub r#type: String,
	pub size: String,
	pub label: String,
	pub format: String,
}

/// A generic provisioner
#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct Provisioner {
	pub r#type: String,

	#[serde(flatten)]
	pub ansible: AnsibleProvisioner,

	#[serde(flatten)]
	pub shell: ShellProvisioner,
}

impl Provisioner {
	pub fn run(&self, ssh: &SshConnection) -> Result<(), Box<dyn Error>> {
		// Check for inline command
		if let Some(command) = &self.shell.inline {
			if ssh.exec(command)? != 0 {
				bail!("Provisioning failed");
			}
		}

		// Check for shell scripts to upload
		for script in &self.shell.scripts {
			ssh.upload(std::fs::read(script)?, ".gb_script")?;

			// Execute it
			ssh.exec(".gb_script")?;
		}

		// Run an ansible playbook
		if let Some(playbook) = &self.ansible.playbook {
			if let Some(code) = Command::new("ansible-playbook")
				.arg("-u")
				.arg(ssh.username.clone())
				.arg("-p")
				.arg(ssh.password.clone())
				.arg(&playbook)
				.status()
				.expect("Failed to launch ansible-playbook")
				.code()
			{}
		}
		Ok(())
	}
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug, Default)]
pub struct AnsibleProvisioner {
	pub playbook: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug, Default)]
pub struct ShellProvisioner {
	pub scripts: Vec<String>,
	pub inline: Option<String>,
}

impl ShellProvisioner {
	/// Create a new shell provisioner with inline command
	pub fn inline(command: &str) -> Provisioner {
		Provisioner {
			r#type: String::from("shell"),
			ansible: AnsibleProvisioner::default(),
			shell: ShellProvisioner {
				inline: Some(command.to_string()),
				scripts: vec![],
			},
		}
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		let result = 2 + 2;
		assert_eq!(result, 4);
	}
}
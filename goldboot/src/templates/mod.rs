use crate::{build::BuildWorker, *};
use serde::{Deserialize, Serialize};

use std::error::Error;
use validator::Validate;

use crate::templates::{
	alpine_linux::AlpineLinuxTemplate, arch_linux::ArchLinuxTemplate, debian::DebianTemplate,
	goldboot_linux::GoldbootLinuxTemplate, mac_os::MacOsTemplate, pop_os::PopOsTemplate,
	steam_deck::SteamDeckTemplate, steam_os::SteamOsTemplate, ubuntu::UbuntuTemplate,
	windows_10::Windows10Template,
};

pub mod alpine_linux;
pub mod arch_linux;
pub mod debian;
pub mod goldboot_linux;
pub mod mac_os;
pub mod pop_os;
pub mod steam_deck;
pub mod steam_os;
pub mod ubuntu;
pub mod windows_10;
pub mod windows_11;
pub mod windows_7;

/// Represents a "base configuration" that users can modify and use to build
/// images.
pub trait Template {
	/// Build an image from the template.
	fn build(&self, context: &BuildWorker) -> Result<(), Box<dyn Error>>;

	/// Whether the template can be combined with others.
	fn is_multiboot(&self) -> bool {
		true
	}

	fn general(&self) -> GeneralContainer;
}

pub trait Promptable {
	fn prompt(
		config: &BuildConfig,
		theme: &dialoguer::theme::ColorfulTheme,
	) -> Result<serde_json::Value, Box<dyn Error>>
	where
		Self: Sized;
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(tag = "base")]
pub enum TemplateBase {
	#[default]
	AlpineLinux,
	ArchLinux,
	Debian,
	GoldbootLinux,
	MacOs,
	PopOs,
	SteamDeck,
	SteamOs,
	Ubuntu,
	Windows10,
}

impl TryFrom<String> for TemplateBase {
	type Error = Box<dyn Error>;
	fn try_from(s: String) -> Result<Self, Self::Error> {
		match s.as_str() {
			"ArchLinux" => Ok(TemplateBase::ArchLinux),
			_ => bail!("Unknown template name"),
		}
	}
}

impl TemplateBase {
	#[rustfmt::skip]
	pub fn parse_template(&self, value: serde_json::Value) -> Result<Box<dyn Template>, Box<dyn Error>> {

		Ok(match self {
			TemplateBase::AlpineLinux   => Box::new(serde_json::from_value::<AlpineLinuxTemplate>(value)?),
			TemplateBase::ArchLinux     => Box::new(serde_json::from_value::<ArchLinuxTemplate>(value)?),
			TemplateBase::Debian        => Box::new(serde_json::from_value::<DebianTemplate>(value)?),
			TemplateBase::GoldbootLinux => Box::new(serde_json::from_value::<GoldbootLinuxTemplate>(value)?),
			TemplateBase::MacOs         => Box::new(serde_json::from_value::<MacOsTemplate>(value)?),
			TemplateBase::PopOs         => Box::new(serde_json::from_value::<PopOsTemplate>(value)?),
			TemplateBase::SteamDeck     => Box::new(serde_json::from_value::<SteamDeckTemplate>(value)?),
			TemplateBase::SteamOs       => Box::new(serde_json::from_value::<SteamOsTemplate>(value)?),
			TemplateBase::Ubuntu        => Box::new(serde_json::from_value::<UbuntuTemplate>(value)?),
			TemplateBase::Windows10     => Box::new(serde_json::from_value::<Windows10Template>(value)?),
			//"windows11"     => Box::new(serde_json::from_value::<Windows11Template>(value)?),
			//"windows7"      => Box::new(serde_json::from_value::<Windows7Template>(value)?),
		})
	}

	#[rustfmt::skip]
	pub fn get_default_template(&self) -> Result<serde_json::Value, Box<dyn Error>> {
		Ok(match self {
			TemplateBase::AlpineLinux    => serde_json::to_value(AlpineLinuxTemplate::default()),
			TemplateBase::ArchLinux      => serde_json::to_value(ArchLinuxTemplate::default()),
			TemplateBase::Debian         => serde_json::to_value(DebianTemplate::default()),
			TemplateBase::GoldbootLinux  => serde_json::to_value(GoldbootLinuxTemplate::default()),
			TemplateBase::MacOs          => serde_json::to_value(MacOsTemplate::default()),
			TemplateBase::PopOs          => serde_json::to_value(PopOsTemplate::default()),
			TemplateBase::SteamDeck      => serde_json::to_value(SteamDeckTemplate::default()),
			TemplateBase::SteamOs        => serde_json::to_value(SteamOsTemplate::default()),
			TemplateBase::Ubuntu         => serde_json::to_value(UbuntuTemplate::default()),
			TemplateBase::Windows10      => serde_json::to_value(Windows10Template::default()),
			//"windows11"      => serde_json::to_value(Windows11Template::default()),
			//"windows7"       => serde_json::to_value(Windows7Template::default()),
		}?)
	}
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct IsoContainer {
	/// The installation media URL
	pub url: String,

	/// A hash of the installation media
	pub checksum: String,
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug, Default)]
pub struct ProvisionersContainer {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provisioners: Option<Vec<serde_json::Value>>,
}

impl ProvisionersContainer {
	pub fn run(&self, ssh: &mut SshConnection) -> Result<(), Box<dyn Error>> {
		if let Some(provisioners) = &self.provisioners {
			for provisioner in provisioners {
				match provisioner.get("type").unwrap().as_str().unwrap() {
					"ansible" => {
						let provisioner: AnsibleProvisioner =
							serde_json::from_value(provisioner.to_owned())?;
						provisioner.run(ssh)?;
					}
					"shell" => {
						let provisioner: ShellProvisioner =
							serde_json::from_value(provisioner.to_owned())?;
						provisioner.run(ssh)?;
					}
					"script" => {
						let provisioner: ScriptProvisioner =
							serde_json::from_value(provisioner.to_owned())?;
						provisioner.run(ssh)?;
					}
					_ => {}
				}
			}
		}
		Ok(())
	}
}

impl Promptable for ProvisionersContainer {
	fn prompt(
		config: &BuildConfig,
		theme: &dialoguer::theme::ColorfulTheme,
	) -> Result<serde_json::Value, Box<dyn Error>> {
		loop {
			if !dialoguer::Confirm::with_theme(theme)
				.with_prompt("Do you want to add a provisioner?")
				.interact()?
			{
				break;
			}

			// Prompt provisioner type
			{
				let provisioner_index = dialoguer::Select::with_theme(theme)
					.with_prompt("Choose provisioner type")
					.default(0)
					.item("Shell script")
					.item("Ansible playbook")
					.interact()?;

				match provisioner_index {
					0 => {}
					1 => {}
					_ => panic!(),
				};
			}
		}
		todo!()
	}
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug, Default)]
pub struct GeneralContainer {
	#[serde(flatten)]
	pub base: TemplateBase,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub qemuargs: Option<Vec<String>>,

	pub storage_size: String,
}

impl GeneralContainer {
	pub fn storage_size_bytes(&self) -> u64 {
		self.storage_size
			.parse::<ubyte::ByteUnit>()
			.unwrap()
			.as_u64()
	}
}

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct RootPasswordContainer {
	#[validate(length(max = 64))]
	pub root_password: String,
}

impl Promptable for RootPasswordContainer {
	fn prompt(
		config: &BuildConfig,
		theme: &dialoguer::theme::ColorfulTheme,
	) -> Result<serde_json::Value, Box<dyn Error>> {
		let root_password = dialoguer::Password::with_theme(theme)
			.with_prompt("Root password")
			.interact()?;

		todo!()
	}
}

impl Default for RootPasswordContainer {
	fn default() -> RootPasswordContainer {
		RootPasswordContainer {
			root_password: crate::random_password(),
		}
	}
}

pub struct LuksContainer {
	/// The LUKS container passphrase
	pub luks_passphrase: String,

	/// Whether the LUKS passphrase will be enrolled in a TPM
	pub tpm: bool,
}

impl Promptable for LuksContainer {
	fn prompt(
		config: &BuildConfig,
		theme: &dialoguer::theme::ColorfulTheme,
	) -> Result<serde_json::Value, Box<dyn Error>> {
		if dialoguer::Confirm::with_theme(theme)
			.with_prompt("Do you want to encrypt the root partition with LUKS?")
			.interact()?
		{
			let luks_password = dialoguer::Password::with_theme(theme)
				.with_prompt("LUKS passphrase")
				.interact()?;
		}

		todo!()
	}
}
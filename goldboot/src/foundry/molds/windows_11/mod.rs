use std::collections::HashMap;

use anyhow::Result;
use dialoguer::theme::Theme;
use goldboot_image::ImageArch;
use serde::{Deserialize, Serialize};
use serde_win_unattend::*;
use tracing::debug;
use validator::Validate;

use crate::{
    cli::prompt::Prompt,
    enter,
    foundry::{
        options::hostname::Hostname,
        qemu::{OsCategory, QemuBuilder},
        sources::ImageSource,
        Foundry, FoundryWorker,
    },
    wait,
};

use super::{CastImage, DefaultSource};

#[derive(Clone, Serialize, Deserialize, Validate, Debug, Default)]
pub struct Windows11 {
    #[serde(flatten)]
    pub hostname: Hostname,
}

// TODO proc macro
impl Prompt for Windows11 {
    fn prompt(&mut self, _foundry: &Foundry, theme: Box<dyn Theme>) -> Result<()> {
        // Prompt for minimal install
        if dialoguer::Confirm::with_theme(&*theme).with_prompt("Perform minimal install? This will remove as many unnecessary programs as possible.").interact()? {

		}
        Ok(())
    }
}

impl DefaultSource for Windows11 {
    fn default_source(&self, _: ImageArch) -> Result<ImageSource> {
        // TODO? https://github.com/pbatard/Fido
        Ok(ImageSource::Iso {
            url: "<TODO>.iso".to_string(),
            checksum: Some("sha256:<TODO>".to_string()),
        })
    }
}

impl CastImage for Windows11 {
    fn cast(&self, worker: &FoundryWorker) -> Result<()> {
        let unattended = UnattendXml {
            xmlns: "urn:schemas-microsoft-com:unattend".into(),
            settings: vec![
                Settings {
                    pass: "windowsPE".into(),
                    component: vec![
                        Component {
                            name: "Microsoft-Windows-International-Core-WinPE".into(),
                            UILanguage: Some("en-US".into()),
                            UserLocale: Some("en-US".into()),
                            SystemLocale: Some("en-US".into()),
                            InputLocale: Some("en-US".into()),
                            SetupUILanguage: Some(SetupUILanguage {
                                UILanguage: "en-US".into(),
                            }),
                            ..Default::default()
                        },
                        Component {
                            name: "Microsoft-Windows-Setup".into(),
                            RunSynchronous: Some(RunSynchronous {
                                RunSynchronousCommand: vec![
                                    RunSynchronousCommand {
                                    Path: r#"cmd /c reg add "HKLM\SYSTEM\Setup\LabConfig" /v "BypassSecureBootCheck" /t REG_DWORD /d 1"#.into(),
                                    Description: None,
                                    Order: "1".into(),
                                    action: Some("add".into()),
                                },
                                    RunSynchronousCommand {
                                    Path: r#"cmd /c reg add "HKLM\SYSTEM\Setup\LabConfig" /v "BypassTPMCheck" /t REG_DWORD /d 1"#.into(),
                                    Description: None,
                                    Order: "2".into(),
                                    action: Some("add".into()),
                                },
                                ],
                            }),
                            DiskConfiguration: Some(DiskConfiguration {
                                WillShowUI: None,
                                Disk: Disk {
                                    CreatePartitions: CreatePartitions {
                                        CreatePartition: vec![
                                            CreatePartition {
                                                Order: "1".into(),
                                                Size: Some("512".into()),
                                                Extend: None,
                                                Type: "Primary".into(),
                                            },
                                            CreatePartition {
                                                Order: "2".into(),
                                                Size: None,
                                                Extend: None,
                                                Type: "Primary".into(),
                                            },
                                        ],
                                    },
                                    ModifyPartitions: ModifyPartitions {
                                        ModifyPartition: vec![
                                            ModifyPartition {
                                                Format: "NTFS".into(),
                                                Label: "System".into(),
                                                Order: "1".into(),
                                                PartitionID: "1".into(),
                                                Letter: None,
                                            },
                                            ModifyPartition {
                                                Format: "NTFS".into(),
                                                Label: "OS".into(),
                                                Order: "2".into(),
                                                PartitionID: "2".into(),
                                                Letter: Some("C".into()),
                                            },
                                        ],
                                    },
                                    WillWipeDisk: "false".into(),
                                    DiskID: "0".into(),
                                },
                            }),
                            ImageInstall: Some(ImageInstall {
                                OSImage: OSImage {
                                    InstallTo: Some(InstallTo {
                                        DiskID: "0".into(),
                                        PartitionID: "2".into(),
                                    }),
                                    WillShowUI: Some("Never".into()),
                                    InstallToAvailablePartition: None,
                                },
                            }),
                            UserData: Some(UserData {
                                AcceptEula: "true".into(),
                                FullName: "test".into(),
                                Organization: "test".into(),
                                ProductKey: ProductKey {
                                    Key: "W269N-WFGWX-YVC9B-4J6C9-T83GX".into(),
                                    WillShowUI: Some("Never".into()),
                                },
                            }),
                            ..Default::default()
                        },
                    ],
                },
                Settings {
                    pass: "specialize".into(),
                    component: vec![Component {
                        name: "Microsoft-Windows-Shell-Setup".into(),
                        ComputerName: Some(self.hostname.hostname.clone()),
                        ..Default::default()
                    }],
                },
            ],
        };
        let unattended_xml = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n{}",
            quick_xml::se::to_string(&unattended)?
        );
        debug!(xml = unattended_xml, "Generated Autounattend.xml");

        let mut qemu = QemuBuilder::new(&worker, OsCategory::Windows)
            .source(&worker.element.source)?
            .floppy_files(HashMap::from([(
                "Autounattend.xml".to_string(),
                unattended_xml.as_bytes().to_vec(),
            )]))?
            .enable_tpm()?
            // .prepare_ssh()?
            .start()?;

        // Copy powershell scripts
        //if let Some(resource) = Resources::get("configure_winrm.ps1") {
        //    std::fs::write(context.join("configure_winrm.ps1"), resource.data)?;
        //}

        // Send boot command
        #[rustfmt::skip]
		qemu.vnc.run(vec![
            // Initial wait
			wait!(4),
			enter!(),
		])?;

        // Wait for SSH
        // let mut ssh = qemu.ssh_wait(context.ssh_port, &self.username, &self.password)?;

        // Shutdown
        // ssh.shutdown("shutdown /s /t 0 /f /d p:4:1")?;
        qemu.shutdown_wait()?;
        Ok(())
    }
}

use crate::cli::prompt::Prompt;
use dialoguer::Password;
use serde::{Deserialize, Serialize};
use std::error::Error;
use validator::Validate;

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct UnixAccountProvisioners {
    pub users: Vec<UnixAccountProvisioner>,
}

impl UnixAccountProvisioners {
    /// Get the root user's password
    pub fn get_root_password(&self) -> Option<String> {
        self.users
            .iter()
            .filter(|u| u.username == "root")
            .map(|u| u.password)
            .next()
    }
}

/// This provisioner configures a UNIX-like user account.
#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct UnixAccountProvisioner {
    #[validate(length(max = 64))]
    pub username: String,

    #[validate(length(max = 64))]
    pub password: String,
}

impl Prompt for UnixAccountProvisioner {
    fn prompt(
        &mut self,
        config: &BuildConfig,
        theme: Box<dyn dialoguer::theme::Theme>,
    ) -> Result<(), Box<dyn Error>> {
        self.password = Password::with_theme(&theme)
            .with_prompt("Root password")
            .interact()?;

        self.validate()?;
        Ok(())
    }
}

impl Default for UnixAccountProvisioner {
    fn default() -> Self {
        Self {
            username: String::from("root"),
            password: crate::random_password(),
        }
    }
}
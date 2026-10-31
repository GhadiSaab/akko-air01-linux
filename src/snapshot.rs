use crate::protocol::{Action, check_profile};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSnapshot {
    pub index: u8,
    pub base: Vec<Action>,
    pub fn_win: Vec<Action>,
    pub fn_mac: Vec<Action>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub device_id: u32,
    pub active_profile: u8,
    pub profiles: Vec<ProfileSnapshot>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == 1, "unsupported snapshot schema");
        ensure!(
            self.device_id == 3013,
            "snapshot is not for supported Air 01 device 3013"
        );
        check_profile(self.active_profile)?;
        ensure!(
            self.profiles.len() == 4,
            "snapshot must contain four profiles"
        );
        let mut seen = [false; 4];
        for p in &self.profiles {
            check_profile(p.index)?;
            ensure!(!seen[p.index as usize], "duplicate profile {}", p.index);
            seen[p.index as usize] = true;
            ensure!(
                p.base.len() == 128 && p.fn_win.len() == 128 && p.fn_mac.len() == 128,
                "profile {} must contain 128 entries per layer",
                p.index
            );
        }
        Ok(())
    }
    pub fn profile(&self, index: u8) -> Result<&ProfileSnapshot> {
        self.profiles
            .iter()
            .find(|p| p.index == index)
            .ok_or_else(|| anyhow::anyhow!("profile {index} absent"))
    }
}

use crate::{
    protocol::{Action, check_profile, detect_gen2},
    snapshot::{ProfileSnapshot, Snapshot},
    transport::Wire,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    Base,
    FnWin,
    FnMac,
}
impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Base => "base",
                Self::FnWin => "fn-win",
                Self::FnMac => "fn-mac",
            }
        )
    }
}
impl std::str::FromStr for Layer {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "base" => Ok(Self::Base),
            "fn-win" => Ok(Self::FnWin),
            "fn-mac" => Ok(Self::FnMac),
            _ => anyhow::bail!("layer must be base, fn-win or fn-mac"),
        }
    }
}
pub struct Board<W: Wire> {
    pub wire: W,
    pub device_id: u32,
    pub wired: bool,
    pub version: u16,
}
impl<W: Wire> Board<W> {
    pub fn connect(mut wire: W, wired: bool) -> Result<Self> {
        let id = wire.query(0x8f, &[])?;
        ensure!(id[0] == 0x8f, "invalid identify response");
        let device_id = u32::from_le_bytes(id[1..5].try_into()?);
        ensure!(
            device_id == 3013,
            "unsupported device ID {device_id}; writes are disabled"
        );
        let options = wire.query(0x89, &[0, 0])?;
        let keys = wire.query(0x8a, &[0, 255, 0, 0])?;
        let other_sleep = wire.query(0x92, &[])?;
        ensure!(
            detect_gen2(&options, &keys, &other_sleep),
            "protocol is unknown or ambiguous; writes are disabled"
        );
        Ok(Self {
            wire,
            device_id,
            wired,
            version: u16::from_le_bytes([id[7], id[8]]),
        })
    }
    pub fn check_identity(&mut self) -> Result<()> {
        let id = self.wire.query(0x8f, &[])?;
        ensure!(
            id[0] == 0x8f && u32::from_le_bytes(id[1..5].try_into()?) == self.device_id,
            "device identity changed"
        );
        Ok(())
    }
    pub fn active_profile(&mut self) -> Result<u8> {
        let r = self.wire.query(0x84, &[])?;
        ensure!(r[0] == 0x84, "invalid profile response");
        check_profile(r[1])?;
        Ok(r[1])
    }
    pub fn page(&mut self, profile: u8, layer: Layer, page: u8) -> Result<[u8; 64]> {
        check_profile(profile)?;
        ensure!(page < 8, "page must be 0..7");
        match layer {
            Layer::Base => self.wire.query(0x8a, &[profile, 255, page, 0]),
            Layer::FnWin => self.wire.query(0x90, &[0, profile, 255, page]),
            Layer::FnMac => self.wire.query(0x90, &[1, profile, 255, page]),
        }
    }
    pub fn matrix(&mut self, profile: u8, layer: Layer) -> Result<Vec<Action>> {
        let mut out = Vec::with_capacity(128);
        for page in 0..8 {
            let r = self.page(profile, layer, page)?;
            out.extend(
                r.chunks_exact(4)
                    .map(|v| Action(v.try_into().expect("four-byte chunk"))),
            );
        }
        Ok(out)
    }
    pub fn snapshot(&mut self) -> Result<Snapshot> {
        self.check_identity()?;
        let active = self.active_profile()?;
        let mut profiles = Vec::with_capacity(4);
        for index in 0..4 {
            profiles.push(ProfileSnapshot {
                index,
                base: self.matrix(index, Layer::Base)?,
                fn_win: self.matrix(index, Layer::FnWin)?,
                fn_mac: self.matrix(index, Layer::FnMac)?,
            });
        }
        ensure!(
            self.active_profile()? == active,
            "profile changed while reading snapshot; try again"
        );
        let s = Snapshot {
            schema: 1,
            device_id: self.device_id,
            active_profile: active,
            profiles,
        };
        s.validate()?;
        Ok(s)
    }
}

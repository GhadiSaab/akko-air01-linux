//! Linux HID access on a worker thread. A timed-out transport is poisoned;
//! no further command is queued after an operation with an unknown outcome.
use crate::protocol::packet;
use anyhow::{Context, Result, bail, ensure};
use fs2::FileExt;
use hidapi::{BusType, HidApi};
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::mpsc::{self, Sender},
    time::Duration,
};

pub trait Wire {
    fn query(&mut self, op: u8, args: &[u8]) -> Result<[u8; 64]>;
    fn send(&mut self, packet: [u8; 65]) -> Result<()>;
    fn wait(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
pub fn decode_report(r: &[u8]) -> Result<[u8; 64]> {
    ensure!(
        r.len() == 65,
        "short HID report: expected 65 bytes, received {}",
        r.len()
    );
    ensure!(r[0] == 0, "unexpected HID report ID {}", r[0]);
    Ok(r[1..].try_into()?)
}
#[derive(Clone, Debug)]
pub struct Device {
    pub path: String,
    pub vid: u16,
    pub pid: u16,
    pub product: String,
    pub wired: bool,
}
pub fn devices() -> Result<Vec<Device>> {
    let api = HidApi::new()?;
    Ok(api
        .device_list()
        .filter(|d| d.vendor_id() == 0x3151 && d.usage_page() == 0xffff && d.usage() == 2)
        .map(|d| Device {
            path: d.path().to_string_lossy().into_owned(),
            vid: d.vendor_id(),
            pid: d.product_id(),
            product: d.product_string().unwrap_or("Akko HID").into(),
            wired: matches!(d.bus_type(), BusType::Usb) && d.product_id() == 0x5002,
        })
        .collect())
}
pub fn state_dir() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(p).join("air01"));
    }
    Ok(
        PathBuf::from(std::env::var_os("HOME").context("HOME is unset")?)
            .join(".local/state/air01"),
    )
}
struct Request {
    packet: [u8; 65],
    read: bool,
    reply: Sender<std::result::Result<Vec<u8>, String>>,
}
pub struct HidWire {
    sender: Sender<Request>,
    poisoned: bool,
    _lock: File,
}
impl HidWire {
    pub fn open(device: &Device) -> Result<Self> {
        let dir = state_dir()?;
        std::fs::create_dir_all(&dir)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("device.lock"))?;
        lock.try_lock_exclusive()
            .context("another air01 process is using the keyboard")?;
        let api = HidApi::new()?;
        let path = std::ffi::CString::new(device.path.as_bytes())?;
        let hid = api
            .open_path(&path)
            .context("cannot open settings interface; see the udev instructions")?;
        let (sender, receiver) = mpsc::channel::<Request>();
        std::thread::Builder::new()
            .name("air01-hid".into())
            .spawn(move || {
                while let Ok(r) = receiver.recv() {
                    let result = (|| -> Result<Vec<u8>> {
                        hid.send_feature_report(&r.packet)?;
                        if !r.read {
                            return Ok(vec![]);
                        }
                        std::thread::sleep(Duration::from_millis(120));
                        let mut response = [0; 65];
                        let n = hid.get_feature_report(&mut response)?;
                        Ok(response[..n].to_vec())
                    })()
                    .map_err(|e| e.to_string());
                    let _ = r.reply.send(result);
                }
            })?;
        Ok(Self {
            sender,
            poisoned: false,
            _lock: lock,
        })
    }
    fn exchange(&mut self, p: [u8; 65], read: bool) -> Result<Vec<u8>> {
        ensure!(
            !self.poisoned,
            "HID operation previously timed out; reconnect before further operations"
        );
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Request {
                packet: p,
                read,
                reply: tx,
            })
            .context("HID worker stopped")?;
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(r) => r.map_err(anyhow::Error::msg),
            Err(e) => {
                self.poisoned = true;
                bail!(
                    "HID operation timed out or worker stopped ({e}); outcome unknown; no further writes sent"
                )
            }
        }
    }
}
impl Wire for HidWire {
    fn query(&mut self, op: u8, args: &[u8]) -> Result<[u8; 64]> {
        ensure!(
            [0x8f, 0x89, 0x8a, 0x92, 0x84, 0x90, 0x87].contains(&op),
            "unsupported query opcode"
        );
        decode_report(&self.exchange(packet(op, args, None)?, true)?)
    }
    fn send(&mut self, p: [u8; 65]) -> Result<()> {
        ensure!(
            p[0] == 0 && [0x0a, 0x10, 0x04, 0x07, 0x12].contains(&p[1]),
            "unsupported write opcode or report ID"
        );
        ensure!(
            p[1..if p[1] == 7 { 10 } else { 9 }]
                .iter()
                .map(|&v| v as u16)
                .sum::<u16>()
                & 255
                == 255,
            "invalid write checksum"
        );
        self.exchange(p, false)?;
        Ok(())
    }
}

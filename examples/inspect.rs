//! Read-only smoke test; never sends configuration writes.
use air01::{
    device::Board,
    protocol::Action,
    transport::{HidWire, devices},
};
fn main() -> anyhow::Result<()> {
    let ds = devices()?;
    for d in &ds {
        println!("{} {:04x}:{:04x} {}", d.path, d.vid, d.pid, d.product);
    }
    let d = ds
        .iter()
        .find(|d| d.wired)
        .ok_or_else(|| anyhow::anyhow!("connect the Air 01 by USB"))?;
    let mut b = Board::connect(HidWire::open(d)?, d.wired)?;
    println!(
        "device={} protocol=Gen2 version={:x}.{:02x}",
        b.device_id,
        b.version >> 8,
        b.version & 255
    );
    let s = b.snapshot()?;
    println!("active profile={}", s.active_profile);
    for p in s.profiles {
        let f = p
            .base
            .iter()
            .enumerate()
            .filter(|(_, a)| **a == Action::FN)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        println!(
            "profile={} Fn={:?} slot5={} slot11={}",
            p.index,
            f,
            p.base[5].label(),
            p.base[11].label()
        );
    }
    Ok(())
}

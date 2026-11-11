// SPDX-License-Identifier: MIT OR Apache-2.0
//! Parsing macOS IORegistry trees (`ioreg -a -r -l -c IOUSBHostDevice`).
//!
//! The tree for a Walkman (R-003 part 1) is:
//! `IOUSBHostDevice` (VID/PID/serial/speed) → … → `IOSCSILogicalUnitNub` (INQUIRY + LUN)
//! → … → `IOMedia` (BSD name `disk4`) → partition `IOMedia` (`disk5s1`).
//! This module is platform-independent so it can be tested on every CI runner;
//! only running `ioreg` is macOS-specific.

use std::collections::BTreeMap;

use plist::{Dictionary, Value};
use serde::{Deserialize, Serialize};

/// Identity of one block device (whole disk or partition) attached over USB.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbMedia {
    /// BSD name, e.g. `disk4` or `disk5s1`.
    pub bsd_name: String,
    /// USB vendor ID.
    pub usb_vid: Option<u16>,
    /// USB product ID.
    pub usb_pid: Option<u16>,
    /// USB serial number.
    pub usb_serial: Option<String>,
    /// USB vendor string.
    pub usb_manufacturer: Option<String>,
    /// USB product string.
    pub usb_product: Option<String>,
    /// USB `bcdDevice` (device release number).
    pub usb_bcd_device: Option<u16>,
    /// USB location ID (port path).
    pub usb_location_id: Option<u32>,
    /// Negotiated link speed in Mbit/s.
    pub usb_speed_mbps: Option<u32>,
    /// SCSI INQUIRY vendor.
    pub scsi_vendor: Option<String>,
    /// SCSI INQUIRY product.
    pub scsi_product: Option<String>,
    /// SCSI INQUIRY revision.
    pub scsi_revision: Option<String>,
    /// SCSI logical unit number.
    pub lun: Option<u8>,
    /// Media size in bytes.
    pub size: Option<u64>,
    /// Whether this is a whole disk (not a partition).
    pub whole: bool,
}

#[derive(Clone, Default)]
struct Ctx {
    usb_vid: Option<u16>,
    usb_pid: Option<u16>,
    usb_serial: Option<String>,
    usb_manufacturer: Option<String>,
    usb_product: Option<String>,
    usb_bcd_device: Option<u16>,
    usb_location_id: Option<u32>,
    usb_speed_mbps: Option<u32>,
    scsi_vendor: Option<String>,
    scsi_product: Option<String>,
    scsi_revision: Option<String>,
    lun: Option<u8>,
}

fn s(d: &Dictionary, k: &str) -> Option<String> {
    d.get(k)
        .and_then(Value::as_string)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

fn u(d: &Dictionary, k: &str) -> Option<u64> {
    d.get(k).and_then(Value::as_unsigned_integer)
}

/// Builds a map from BSD name to USB/SCSI identity from an `ioreg -a` plist.
#[must_use]
pub fn parse(root: &Value) -> BTreeMap<String, UsbMedia> {
    let mut out = BTreeMap::new();
    let nodes: Vec<&Value> = match root {
        Value::Array(a) => a.iter().collect(),
        other => vec![other],
    };
    for n in nodes {
        walk(n, &Ctx::default(), &mut out);
    }
    out
}

fn walk(node: &Value, ctx: &Ctx, out: &mut BTreeMap<String, UsbMedia>) {
    let Some(d) = node.as_dictionary() else {
        return;
    };
    let class = s(d, "IOObjectClass").unwrap_or_default();
    let mut ctx = ctx.clone();

    if class == "IOUSBHostDevice" || class == "IOUSBDevice" {
        ctx.usb_vid = u(d, "idVendor").and_then(|v| u16::try_from(v).ok());
        ctx.usb_pid = u(d, "idProduct").and_then(|v| u16::try_from(v).ok());
        ctx.usb_serial = s(d, "USB Serial Number").or_else(|| s(d, "kUSBSerialNumberString"));
        ctx.usb_manufacturer = s(d, "USB Vendor Name").or_else(|| s(d, "kUSBVendorString"));
        ctx.usb_product = s(d, "USB Product Name").or_else(|| s(d, "kUSBProductString"));
        ctx.usb_bcd_device = u(d, "bcdDevice").and_then(|v| u16::try_from(v).ok());
        ctx.usb_location_id = u(d, "locationID").and_then(|v| u32::try_from(v).ok());
        ctx.usb_speed_mbps =
            u(d, "UsbLinkSpeed").and_then(|bps| u32::try_from(bps / 1_000_000).ok());
    }
    if d.contains_key("SCSI Logical Unit Number") && d.contains_key("Vendor Identification") {
        ctx.scsi_vendor = s(d, "Vendor Identification");
        ctx.scsi_product = s(d, "Product Identification");
        ctx.scsi_revision = s(d, "Product Revision Level");
        ctx.lun = u(d, "SCSI Logical Unit Number").and_then(|v| u8::try_from(v).ok());
    }
    if class == "IOMedia"
        && let Some(bsd) = s(d, "BSD Name")
    {
        let whole = d.get("Whole").and_then(Value::as_boolean).unwrap_or(false);
        out.insert(
            bsd.clone(),
            UsbMedia {
                bsd_name: bsd,
                usb_vid: ctx.usb_vid,
                usb_pid: ctx.usb_pid,
                usb_serial: ctx.usb_serial.clone(),
                usb_manufacturer: ctx.usb_manufacturer.clone(),
                usb_product: ctx.usb_product.clone(),
                usb_bcd_device: ctx.usb_bcd_device,
                usb_location_id: ctx.usb_location_id,
                usb_speed_mbps: ctx.usb_speed_mbps,
                scsi_vendor: ctx.scsi_vendor.clone(),
                scsi_product: ctx.scsi_product.clone(),
                scsi_revision: ctx.scsi_revision.clone(),
                lun: ctx.lun,
                size: u(d, "Size"),
                whole,
            },
        );
    }
    if let Some(children) = d.get("IORegistryEntryChildren").and_then(Value::as_array) {
        for c in children {
            walk(c, &ctx, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZX300: &[u8] = include_bytes!("../../../fixtures/probe/macos/zx300.ioreg.plist");

    #[test]
    fn maps_both_luns_and_the_partition_of_a_real_zx300() {
        let v = Value::from_reader(std::io::Cursor::new(ZX300)).unwrap();
        let map = parse(&v);
        let internal = &map["disk4"];
        assert_eq!(internal.usb_vid, Some(0x054c));
        assert_eq!(internal.usb_pid, Some(0x0c70));
        assert_eq!(internal.usb_product.as_deref(), Some("WALKMAN"));
        assert_eq!(internal.usb_manufacturer.as_deref(), Some("Sony"));
        assert_eq!(internal.scsi_vendor.as_deref(), Some("SONY"));
        assert_eq!(internal.scsi_product.as_deref(), Some("WALKMAN"));
        assert_eq!(internal.scsi_revision.as_deref(), Some("1.00"));
        assert_eq!(internal.lun, Some(0));
        assert_eq!(internal.usb_speed_mbps, Some(480));
        assert!(internal.whole);
        // The SD card is LUN 1 of the same device; its exFAT partition inherits the identity.
        let card = &map["disk5s1"];
        assert_eq!(card.lun, Some(1));
        assert_eq!(card.usb_serial, internal.usb_serial);
        assert!(!card.whole);
    }
}

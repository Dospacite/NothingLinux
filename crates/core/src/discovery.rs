use bluer::{Adapter, AdapterEvent};
use futures::{Stream, StreamExt};
use nothing_protocol::DeviceDescriptor;
use std::{collections::HashMap, pin::Pin, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

pub const NOTHING_SERVICE_UUID: &str = "aeac4a03-dff5-498f-843a-34487cf133eb";
const NOTHING_COMPANY_ID: u16 = 0x0ccb;
const B171_MANUFACTURER_PREFIX: [u8; 4] = [0x01, 0x01, 0xb1, 0x71];

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("BlueZ error: {0}")]
    Bluez(#[from] bluer::Error),
    #[error("invalid built-in vendor UUID")]
    VendorUuid,
    #[error(
        "Nothing Ear (2024) is nearby but not paired. Pair it in your desktop Bluetooth settings, keep an earbud out of the case, and Nothing Linux will reconnect automatically."
    )]
    PairingRequired,
    #[error("BlueZ event stream ended")]
    StreamEnded,
}

pub async fn discover_paired(adapter: &Adapter) -> Result<Vec<DeviceDescriptor>, DiscoveryError> {
    let vendor = Uuid::from_str(NOTHING_SERVICE_UUID).map_err(|_| DiscoveryError::VendorUuid)?;
    let mut result = Vec::new();
    for address in adapter.device_addresses().await? {
        if let DiscoveryMatch::Paired(device) = inspect_device(adapter, address, &vendor).await? {
            result.push(device);
        }
    }
    Ok(result)
}

pub async fn monitor_connections(
    adapter: &Adapter,
) -> Result<Pin<Box<dyn Stream<Item = AdapterEvent> + Send>>, DiscoveryError> {
    Ok(Box::pin(adapter.events().await?))
}

pub async fn wait_for_vendor_device(adapter: &Adapter) -> Result<DeviceDescriptor, DiscoveryError> {
    let known = discover_paired(adapter).await?;
    if let Some(device) = preferred_device(known) {
        return Ok(device);
    }

    // `Adapter::events` only reports a scan that another application has
    // already started. Own a discovery session so a newly paired earbud can be
    // found even when the desktop settings panel is closed.
    let vendor = Uuid::from_str(NOTHING_SERVICE_UUID).map_err(|_| DiscoveryError::VendorUuid)?;
    let mut events = adapter.discover_devices_with_changes().await?;
    while let Some(event) = events.next().await {
        if let AdapterEvent::DeviceAdded(address) = event {
            match inspect_device(adapter, address, &vendor).await? {
                DiscoveryMatch::Paired(device) => return Ok(device),
                DiscoveryMatch::NearbyUnpaired => return Err(DiscoveryError::PairingRequired),
                DiscoveryMatch::Other => {}
            }
        }
    }
    Err(DiscoveryError::StreamEnded)
}

enum DiscoveryMatch {
    Paired(DeviceDescriptor),
    NearbyUnpaired,
    Other,
}

async fn inspect_device(
    adapter: &Adapter,
    address: bluer::Address,
    vendor: &Uuid,
) -> Result<DiscoveryMatch, DiscoveryError> {
    let device = adapter.device(address)?;
    let alias = device
        .alias()
        .await
        .unwrap_or_else(|_| "Nothing audio device".into());
    let manufacturer_data = device
        .manufacturer_data()
        .await
        .unwrap_or(None)
        .unwrap_or_default();
    let b171_advertisement = is_b171_advertisement(&alias, &manufacturer_data);

    if !device.is_paired().await.unwrap_or(false) {
        return Ok(if b171_advertisement {
            DiscoveryMatch::NearbyUnpaired
        } else {
            DiscoveryMatch::Other
        });
    }

    let uuids = device.uuids().await.unwrap_or(None).unwrap_or_default();
    let has_vendor_service = uuids.contains(vendor);
    // BlueZ can omit an RFCOMM service UUID from a cached paired device until
    // the next SDP refresh. The canonical B171 alias is a safe fallback:
    // protocol activation still verifies the model before enabling writes.
    if !has_vendor_service && !has_b171_alias(&alias) && !b171_advertisement {
        return Ok(DiscoveryMatch::Other);
    }

    Ok(DiscoveryMatch::Paired(DeviceDescriptor {
        address: address.to_string(),
        alias,
        model: b171_advertisement.then_some("B171".into()),
        paired: true,
        connected: device.is_connected().await.unwrap_or(false),
        vendor_service: has_vendor_service,
    }))
}

fn preferred_device(devices: Vec<DeviceDescriptor>) -> Option<DeviceDescriptor> {
    devices
        .iter()
        .find(|device| device.connected)
        .cloned()
        .or_else(|| devices.into_iter().next())
}

fn is_b171_advertisement(alias: &str, manufacturer_data: &HashMap<u16, Vec<u8>>) -> bool {
    has_b171_alias(alias)
        && manufacturer_data
            .get(&NOTHING_COMPANY_ID)
            .is_some_and(|data| data.starts_with(&B171_MANUFACTURER_PREFIX))
}

fn has_b171_alias(alias: &str) -> bool {
    alias.trim().eq_ignore_ascii_case("Nothing Ear")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_uuid_is_stable_and_valid() {
        assert!(Uuid::parse_str(NOTHING_SERVICE_UUID).is_ok());
    }

    #[test]
    fn recognizes_the_b171_advertisement_signature() {
        let mut manufacturer_data = HashMap::new();
        manufacturer_data.insert(NOTHING_COMPANY_ID, B171_MANUFACTURER_PREFIX.to_vec());
        assert!(is_b171_advertisement("Nothing Ear", &manufacturer_data));
        assert!(!is_b171_advertisement(
            "Nothing Ear (a)",
            &manufacturer_data
        ));
    }

    #[test]
    fn prefers_an_already_connected_paired_device() {
        let devices = vec![
            DeviceDescriptor {
                address: "00:00:00:00:00:01".into(),
                alias: "Nothing Ear".into(),
                model: Some("B171".into()),
                paired: true,
                connected: false,
                vendor_service: true,
            },
            DeviceDescriptor {
                address: "00:00:00:00:00:02".into(),
                alias: "Nothing Ear".into(),
                model: Some("B171".into()),
                paired: true,
                connected: true,
                vendor_service: true,
            },
        ];
        assert_eq!(
            preferred_device(devices).map(|device| device.address),
            Some("00:00:00:00:00:02".into())
        );
    }
}

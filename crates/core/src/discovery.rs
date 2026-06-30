use bluer::{Adapter, AdapterEvent};
use futures::{Stream, StreamExt};
use nothing_protocol::DeviceDescriptor;
use std::{pin::Pin, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

pub const NOTHING_SERVICE_UUID: &str = "aeac4a03-dff5-498f-843a-34487cf133eb";

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("BlueZ error: {0}")]
    Bluez(#[from] bluer::Error),
    #[error("invalid built-in vendor UUID")]
    VendorUuid,
    #[error("BlueZ event stream ended")]
    StreamEnded,
}

pub async fn discover_paired(adapter: &Adapter) -> Result<Vec<DeviceDescriptor>, DiscoveryError> {
    let vendor = Uuid::from_str(NOTHING_SERVICE_UUID).map_err(|_| DiscoveryError::VendorUuid)?;
    let mut result = Vec::new();
    for address in adapter.device_addresses().await? {
        let device = adapter.device(address)?;
        if !device.is_paired().await.unwrap_or(false) {
            continue;
        }
        let uuids = device.uuids().await.unwrap_or(None).unwrap_or_default();
        let has_vendor = uuids.contains(&vendor);
        if !has_vendor {
            continue;
        }
        result.push(DeviceDescriptor {
            address: address.to_string(),
            alias: device
                .alias()
                .await
                .unwrap_or_else(|_| "Nothing audio device".into()),
            model: None,
            paired: true,
            connected: device.is_connected().await.unwrap_or(false),
            vendor_service: true,
        });
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
    if let Some(device) = known
        .iter()
        .find(|device| device.connected)
        .cloned()
        .or_else(|| known.into_iter().next())
    {
        return Ok(device);
    }
    let mut events = adapter.events().await?;
    while events.next().await.is_some() {
        let known = discover_paired(adapter).await?;
        if let Some(device) = known
            .iter()
            .find(|device| device.connected)
            .cloned()
            .or_else(|| known.into_iter().next())
        {
            return Ok(device);
        }
    }
    Err(DiscoveryError::StreamEnded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vendor_uuid_is_stable_and_valid() {
        assert!(Uuid::parse_str(NOTHING_SERVICE_UUID).is_ok());
    }
}

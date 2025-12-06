//! HCS Layer management - for creating sandboxes without Docker

use std::ffi::c_void;
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        Foundation::HANDLE,
        System::HostComputeSystem::*,
    },
};
use crate::Result;

/// Layer management for HCS containers
pub struct Layer {
    path: String,
}

impl Layer {
    /// Create a new layer at the given path
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// Get the layer path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Setup a base OS layer from the host Windows installation
    /// This creates a copy-on-write view of your Windows - no Docker images needed!
    pub fn setup_base_os_layer(layer_path: &str, vhd_handle: HANDLE) -> Result<()> {
        unsafe {
            let path = HSTRING::from(layer_path);
            let options = HSTRING::from("{}");

            HcsSetupBaseOSLayer(
                PCWSTR(path.as_ptr()),
                vhd_handle,
                PCWSTR(options.as_ptr()),
            )?;

            Ok(())
        }
    }

    /// Setup base OS from a volume path (alternative to VHD)
    pub fn setup_base_os_volume(layer_path: &str, volume_path: &str) -> Result<()> {
        unsafe {
            let layer = HSTRING::from(layer_path);
            let volume = HSTRING::from(volume_path);
            let options = HSTRING::from("{}");

            HcsSetupBaseOSVolume(
                PCWSTR(layer.as_ptr()),
                PCWSTR(volume.as_ptr()),
                PCWSTR(options.as_ptr()),
            )?;

            Ok(())
        }
    }

    /// Initialize a writable layer on top of base layers
    pub fn initialize_writable_layer(
        writable_layer_path: &str,
        layer_data: &str,
    ) -> Result<()> {
        unsafe {
            let path = HSTRING::from(writable_layer_path);
            let data = HSTRING::from(layer_data);
            let options = HSTRING::from("{}");

            HcsInitializeWritableLayer(
                PCWSTR(path.as_ptr()),
                PCWSTR(data.as_ptr()),
                PCWSTR(options.as_ptr()),
            )?;

            Ok(())
        }
    }

    /// Format a VHD for use as a writable layer
    pub fn format_writable_layer_vhd(vhd_handle: HANDLE) -> Result<()> {
        unsafe {
            HcsFormatWritableLayerVhd(vhd_handle)?;
            Ok(())
        }
    }

    /// Get the mount path of a layer VHD
    pub fn get_layer_mount_path(vhd_handle: HANDLE) -> Result<String> {
        unsafe {
            let path = HcsGetLayerVhdMountPath(vhd_handle)?;
            let result = path.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(path.as_ptr() as *const c_void));
            Ok(result)
        }
    }

    /// Attach storage filter to a layer
    pub fn attach_storage_filter(layer_path: &str, layer_data: &str) -> Result<()> {
        unsafe {
            let path = HSTRING::from(layer_path);
            let data = HSTRING::from(layer_data);

            HcsAttachLayerStorageFilter(
                PCWSTR(path.as_ptr()),
                PCWSTR(data.as_ptr()),
            )?;

            Ok(())
        }
    }

    /// Detach storage filter from a layer
    pub fn detach_storage_filter(layer_path: &str) -> Result<()> {
        unsafe {
            let path = HSTRING::from(layer_path);
            HcsDetachLayerStorageFilter(PCWSTR(path.as_ptr()))?;
            Ok(())
        }
    }

    /// Destroy a layer
    pub fn destroy(layer_path: &str) -> Result<()> {
        unsafe {
            let path = HSTRING::from(layer_path);
            HcsDestroyLayer(PCWSTR(path.as_ptr()))?;
            Ok(())
        }
    }
}

/// Helper functions for creating VHDs using PowerShell
pub mod vhd {
    use crate::{Error, Result};
    use std::process::Command;

    /// Create a new VHDX file
    pub fn create(path: &str, size_gb: u64) -> Result<()> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("New-VHD -Path '{}' -SizeBytes {}GB -Dynamic", path, size_gb),
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::Layer(format!(
                "Failed to create VHD: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    /// Mount a VHDX and return the disk number
    pub fn mount(path: &str) -> Result<u32> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("$vhd = Mount-VHD -Path '{}' -Passthru; $vhd.DiskNumber", path),
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::Layer(format!(
                "Failed to mount VHD: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let disk_num: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .map_err(|_| Error::Layer("Failed to parse disk number".into()))?;

        Ok(disk_num)
    }

    /// Dismount a VHDX
    pub fn dismount(path: &str) -> Result<()> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Dismount-VHD -Path '{}'", path),
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::Layer(format!(
                "Failed to dismount VHD: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    /// Initialize a VHD with GPT and NTFS
    pub fn initialize_and_format(disk_number: u32) -> Result<String> {
        let script = format!(r#"
            $disk = Get-Disk -Number {}
            $disk | Initialize-Disk -PartitionStyle GPT -PassThru | Out-Null
            $part = $disk | New-Partition -UseMaximumSize -AssignDriveLetter
            $part | Format-Volume -FileSystem NTFS -NewFileSystemLabel 'SandboxLayer' -Confirm:$false | Out-Null
            $part.DriveLetter
        "#, disk_number);

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()?;

        if !output.status.success() {
            return Err(Error::Layer(format!(
                "Failed to initialize VHD: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let drive = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(format!("{}:", drive))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_new() {
        let layer = Layer::new("test-layer");
        assert_eq!(layer.path(), "test-layer");
    }
}

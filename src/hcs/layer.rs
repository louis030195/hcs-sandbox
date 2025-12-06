//! Container layer management - the Docker-free path
//! 
//! Windows Sandbox approach:
//! 1. Create a VHD for the writable layer
//! 2. HcsSetupBaseOSLayer - creates dynamic base from host OS (no Docker images needed!)
//! 3. Mount and use as container root filesystem

use std::ffi::c_void;
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::{
        Foundation::HANDLE,
        System::HostComputeSystem::*,
    },
};
use super::system::Result;

/// Setup a base OS layer from the host Windows installation
/// This is the key to running containers without Docker!
/// 
/// The layer_path is where the dynamic base layer metadata will be stored
/// The vhd_handle is the writable layer VHD (create using diskpart or PowerShell)
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

/// Alternative: Setup base OS from a volume path directly
/// volume_path should be like "\?\Volume{guid}\" or "C:\"
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
    layer_data: &str, // JSON with parent layer info
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

/// Attach the storage filter driver to a layer
pub fn attach_layer_storage_filter(layer_path: &str, layer_data: &str) -> Result<()> {
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

/// Detach the storage filter from a layer
pub fn detach_layer_storage_filter(layer_path: &str) -> Result<()> {
    unsafe {
        let path = HSTRING::from(layer_path);
        HcsDetachLayerStorageFilter(PCWSTR(path.as_ptr()))?;
        Ok(())
    }
}

/// Destroy a layer and clean up
pub fn destroy_layer(layer_path: &str) -> Result<()> {
    unsafe {
        let path = HSTRING::from(layer_path);
        HcsDestroyLayer(PCWSTR(path.as_ptr()))?;
        Ok(())
    }
}

/// Helper: Create a VHDX using PowerShell (simpler than raw Win32 API)
/// Returns the path to the created VHDX
pub fn create_vhdx_powershell(path: &str, size_gb: u64) -> std::io::Result<()> {
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "New-VHD -Path '{}' -SizeBytes {}GB -Dynamic",
                path, size_gb
            ),
        ])
        .output()?;
    
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(())
}

/// Helper: Mount a VHDX and return the disk number
pub fn mount_vhdx_powershell(path: &str) -> std::io::Result<String> {
    let output = std::process::Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "$vhd = Mount-VHD -Path '{}' -Passthru; $vhd.DiskNumber",
                path
            ),
        ])
        .output()?;
    
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

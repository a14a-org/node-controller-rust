use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use rdma_sys::*;
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::str;

fn main() -> Result<()> {
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Starting RDMA capability detection for Thunderbolt 5 on Apple Silicon");
    info!("=================================================================");

    // Step 1: Check for RDMA devices
    info!("Checking for RDMA devices...");
    match detect_rdma_devices() {
        Ok(devices) => {
            if devices.is_empty() {
                info!("❌ No RDMA devices found");
            } else {
                info!("✅ Found {} RDMA device(s):", devices.len());
                for (i, device) in devices.iter().enumerate() {
                    info!("  Device #{}: {}", i + 1, device);
                }
            }
        }
        Err(e) => {
            error!("❌ Error detecting RDMA devices: {}", e);
        }
    }

    // Step 2: Check RDMA capabilities
    info!("\nChecking RDMA capabilities...");
    if let Err(e) = check_rdma_capabilities() {
        error!("❌ Error checking RDMA capabilities: {}", e);
    }

    // Step 3: Print system information
    info!("\nSystem information:");
    print_system_info();

    // Step 4: Try RDMA memory registration
    info!("\nTesting RDMA memory registration...");
    if let Err(e) = test_rdma_memory_registration() {
        error!("❌ Error testing RDMA memory registration: {}", e);
        info!("This likely means RDMA is not fully supported on this system.");
    }

    // Final assessment
    info!("\n=================================================================");
    info!("RDMA Support Assessment:");
    assess_rdma_support()
}

/// Detect available RDMA devices
fn detect_rdma_devices() -> Result<Vec<String>> {
    // Safe container for device names
    let mut devices = Vec::new();

    unsafe {
        // Get device list
        let mut num_devices: i32 = 0;
        let device_list = ibv_get_device_list(&mut num_devices);
        if device_list.is_null() {
            if num_devices == 0 {
                return Ok(devices); // No devices found, but not an error
            }
            return Err(anyhow!("Failed to get device list"));
        }

        // Free the device list when going out of scope
        let _device_list_guard = DeviceListGuard(device_list);

        // Iterate through devices
        for i in 0..num_devices as isize {
            let device = *device_list.offset(i);
            if !device.is_null() {
                let device_name = ibv_get_device_name(device);
                if !device_name.is_null() {
                    let name = CStr::from_ptr(device_name).to_string_lossy().into_owned();
                    devices.push(name);
                }
            }
        }
    }

    Ok(devices)
}

/// Test RDMA capabilities
fn check_rdma_capabilities() -> Result<()> {
    let devices = detect_rdma_devices()?;
    if devices.is_empty() {
        info!("❌ No RDMA devices to check capabilities for");
        return Ok(());
    }

    for device_name in devices {
        // Open a device context
        let name_cstring = CString::new(device_name.clone())?;
        unsafe {
            let device = ibv_get_device_by_name(name_cstring.as_ptr());
            if device.is_null() {
                warn!("❌ Could not find device: {}", device_name);
                continue;
            }

            let context = ibv_open_device(device);
            if context.is_null() {
                warn!("❌ Could not open device: {}", device_name);
                continue;
            }

            // Get device attributes
            let mut device_attr: ibv_device_attr = mem::zeroed();
            let ret = ibv_query_device(context, &mut device_attr);
            if ret != 0 {
                warn!("❌ Could not query device attributes for {}", device_name);
                ibv_close_device(context);
                continue;
            }

            // Print device capabilities
            info!("✅ Device {} capabilities:", device_name);
            info!("  • Max QPs: {}", device_attr.max_qp);
            info!("  • Max CQs: {}", device_attr.max_cq);
            info!("  • Max MRs: {}", device_attr.max_mr);
            info!("  • Max PD: {}", device_attr.max_pd);
            info!("  • Max QP WRs: {}", device_attr.max_qp_wr);
            info!("  • Max SGE: {}", device_attr.max_sge);

            // Close device
            ibv_close_device(context);
        }
    }

    Ok(())
}

/// Print system information
fn print_system_info() {
    // Print OS version
    if let Ok(os_type) = sys_info::os_type() {
        if let Ok(os_release) = sys_info::os_release() {
            info!("OS: {} {}", os_type, os_release);
        }
    }

    // Print CPU info
    if let Ok(cpu_num) = sys_info::cpu_num() {
        if let Ok(cpu_speed) = sys_info::cpu_speed() {
            info!("CPU: {} cores at {} MHz", cpu_num, cpu_speed);
        }
    }

    // Check if running on Apple Silicon
    #[cfg(target_arch = "aarch64")]
    #[cfg(target_os = "macos")]
    {
        info!("Running on Apple Silicon (aarch64)");
    }

    // Print memory info
    if let Ok(mem_info) = sys_info::mem_info() {
        let total_gb = mem_info.total as f64 / 1024.0 / 1024.0;
        info!("Memory: {:.2} GB total", total_gb);
    }

    // Check Thunderbolt interfaces (simplified)
    if cfg!(target_os = "macos") {
        // Use system_profiler to detect Thunderbolt on macOS
        use std::process::Command;
        let output = Command::new("system_profiler")
            .arg("SPThunderboltDataType")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("Thunderbolt") {
                    info!("Thunderbolt: Detected");
                    // Extract version information if possible
                    if stdout.contains("Thunderbolt 5") {
                        info!("Thunderbolt Version: 5");
                    } else if stdout.contains("Thunderbolt 4") {
                        info!("Thunderbolt Version: 4");
                    } else if stdout.contains("Thunderbolt 3") {
                        info!("Thunderbolt Version: 3");
                    }
                } else {
                    info!("Thunderbolt: Not detected");
                }
            }
            _ => {
                info!("Thunderbolt: Status unknown (could not query system_profiler)");
            }
        }
    }
}

/// Test if RDMA memory registration works
fn test_rdma_memory_registration() -> Result<()> {
    let devices = detect_rdma_devices()?;
    if devices.is_empty() {
        return Err(anyhow!("No RDMA devices available for memory registration test"));
    }

    // Use the first device for testing
    let device_name = &devices[0];
    let name_cstring = CString::new(device_name.clone())?;
    
    unsafe {
        // Get device
        let device = ibv_get_device_by_name(name_cstring.as_ptr());
        if device.is_null() {
            return Err(anyhow!("Could not find device: {}", device_name));
        }

        // Open device
        let context = ibv_open_device(device);
        if context.is_null() {
            return Err(anyhow!("Could not open device: {}", device_name));
        }

        // Allocate a protection domain
        let pd = ibv_alloc_pd(context);
        if pd.is_null() {
            ibv_close_device(context);
            return Err(anyhow!("Could not allocate protection domain"));
        }

        // Create a small buffer for testing
        let buffer_size = 4096;
        let mut buffer = vec![0u8; buffer_size];

        // Try to register memory region
        let mr = ibv_reg_mr(
            pd,
            buffer.as_mut_ptr() as *mut std::os::raw::c_void,
            buffer_size,
            (ibv_access_flags::IBV_ACCESS_LOCAL_WRITE | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE | ibv_access_flags::IBV_ACCESS_REMOTE_READ) as i32,
        );

        if mr.is_null() {
            ibv_dealloc_pd(pd);
            ibv_close_device(context);
            return Err(anyhow!("Failed to register memory region"));
        }

        // Success! Cleanup
        info!("✅ Successfully registered RDMA memory region");
        info!("  • Buffer size: {} bytes", buffer_size);
        info!("  • LKey: {}", (*mr).lkey);
        info!("  • RKey: {}", (*mr).rkey);

        ibv_dereg_mr(mr);
        ibv_dealloc_pd(pd);
        ibv_close_device(context);
    }

    Ok(())
}

/// Final assessment of RDMA support
fn assess_rdma_support() -> Result<()> {
    let devices = detect_rdma_devices()?;
    
    if devices.is_empty() {
        info!("❌ RDMA is NOT supported on this system");
        info!("Recommendation: Use optimized TCP for file transfers");
        return Ok(());
    }

    // Try to perform a basic RDMA operation to verify functionality
    match test_rdma_memory_registration() {
        Ok(_) => {
            info!("✅ RDMA appears to be FULLY SUPPORTED on this system");
            info!("Recommendation: Proceed with RDMA implementation for maximum throughput");
        }
        Err(_) => {
            info!("⚠️ RDMA devices detected but functionality appears LIMITED");
            info!("Recommendation: Implement both RDMA and optimized TCP options");
        }
    }

    Ok(())
}

// RAII-style guard for freeing the device list
struct DeviceListGuard(*mut *mut ibv_device);

impl Drop for DeviceListGuard {
    fn drop(&mut self) {
        unsafe {
            ibv_free_device_list(self.0);
        }
    }
} 
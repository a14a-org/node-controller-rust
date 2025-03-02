use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Starting RDMA capability detection for Thunderbolt 5 on Apple Silicon");
    info!("=================================================================");

    // Check system details
    info!("\nSystem information:");
    check_system_info().await?;

    // Check if we're on macOS
    let os_type = std::env::consts::OS;
    if os_type == "macos" {
        info!("\nRunning on macOS. Checking for RDMA prerequisites...");
        check_rdma_prerequisites_macos().await?;
    } else {
        info!("\nRunning on {}. Checking for RDMA prerequisites...", os_type);
        check_rdma_prerequisites_linux().await?;
    }

    // Try to detect RDMA devices via system commands
    info!("\nAttempting to detect RDMA devices via system tools...");
    detect_rdma_devices().await?;

    info!("\n=================================================================");
    info!("Final assessment:");
    assess_rdma_support().await?;
    
    Ok(())
}

async fn check_system_info() -> Result<()> {
    // Check OS type and version
    let os_type = std::env::consts::OS;
    let os_family = std::env::consts::FAMILY;
    let arch = std::env::consts::ARCH;

    info!("OS: {} (family: {})", os_type, os_family);
    info!("Architecture: {}", arch);
    
    // If on macOS, get more detailed info using system_profiler
    if os_type == "macos" {
        let output = Command::new("sw_vers")
            .output()
            .map_err(|e| anyhow!("Failed to execute sw_vers: {}", e))?;
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                info!("  {}", line);
            }
        }

        // Check if running on Apple Silicon
        if arch == "aarch64" {
            info!("Running on Apple Silicon");
        }

        // Get processor info
        let output = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .map_err(|e| anyhow!("Failed to get CPU info: {}", e))?;
        
        if output.status.success() {
            let cpu_info = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info!("CPU: {}", cpu_info);
        }

        // Check Thunderbolt interfaces
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
                    
                    // Print entire thunderbolt section
                    debug!("Detailed Thunderbolt info:\n{}", stdout);
                } else {
                    info!("Thunderbolt: Not detected");
                }
            }
            _ => {
                info!("Thunderbolt: Status unknown (could not query system_profiler)");
            }
        }
    } else if os_type == "linux" {
        // Get Linux distribution info
        if let Ok(output) = Command::new("lsb_release").arg("-a").output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    info!("  {}", line);
                }
            }
        }

        // Get CPU info
        if let Ok(output) = Command::new("cat").arg("/proc/cpuinfo").output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let model_name_line = output_str.lines()
                    .find(|line| line.contains("model name"));
                
                if let Some(line) = model_name_line {
                    if let Some(idx) = line.find(':') {
                        let cpu_info = line[idx+1..].trim();
                        info!("CPU: {}", cpu_info);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn check_rdma_prerequisites_macos() -> Result<()> {
    info!("Checking for RDMA libraries on macOS...");
    
    // Check if Homebrew is installed (common way to install libraries on macOS)
    let brew_check = Command::new("which")
        .arg("brew")
        .output()
        .map_err(|e| anyhow!("Failed to execute 'which brew': {}", e))?;
    
    if !brew_check.status.success() {
        warn!("❌ Homebrew not found. It's recommended for installing libraries.");
        info!("  You can install it from https://brew.sh/");
    } else {
        info!("✅ Homebrew is installed");
        
        // Check for pkg-config
        let pkg_config_check = Command::new("which")
            .arg("pkg-config")
            .output()
            .map_err(|e| anyhow!("Failed to check for pkg-config: {}", e))?;
        
        if !pkg_config_check.status.success() {
            warn!("❌ pkg-config not found. It's needed for locating libraries.");
            info!("  Consider installing with 'brew install pkg-config'");
        } else {
            info!("✅ pkg-config is installed");
        }
    }
    
    // Check if any RDMA-related libraries exist
    info!("Searching for RDMA-related libraries...");
    
    // Check in common library locations
    let lib_dirs = [
        "/usr/local/lib",
        "/opt/homebrew/lib",
        "/usr/lib",
    ];
    
    let rdma_lib_patterns = [
        "librdmacm*",
        "libibverbs*",
        "libfabric*",  // Sometimes used for RDMA on different platforms
    ];
    
    let mut found_any = false;
    
    for dir in &lib_dirs {
        for pattern in &rdma_lib_patterns {
            let find_cmd = format!("find {} -name \"{}\" 2>/dev/null", dir, pattern);
            
            let output = Command::new("sh")
                .arg("-c")
                .arg(&find_cmd)
                .output();
            
            if let Ok(output) = output {
                if output.status.success() && !output.stdout.is_empty() {
                    let libs = String::from_utf8_lossy(&output.stdout);
                    for lib in libs.lines() {
                        info!("  Found RDMA-related library: {}", lib);
                        found_any = true;
                    }
                }
            }
        }
    }
    
    if !found_any {
        warn!("❌ No RDMA libraries found on this macOS system");
        info!("  RDMA libraries are typically not available on macOS by default");
        info!("  You might need custom drivers or hardware support to enable RDMA");
    }
    
    // Check for RDMA devices using command line tools that might be available
    let commands_to_try = [
        "ibv_devices",      // From libibverbs
        "rdma_cm_ping",     // From librdmacm
        "fi_info",          // From libfabric
    ];
    
    for cmd in &commands_to_try {
        let cmd_check = Command::new("which")
            .arg(cmd)
            .output();
        
        if let Ok(output) = cmd_check {
            if output.status.success() {
                info!("✅ Found RDMA utility: {}", cmd);
                
                // Try running the command to see if it works
                let cmd_output = Command::new(cmd)
                    .output();
                
                if let Ok(output) = cmd_output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if !output_str.trim().is_empty() {
                        info!("  Output from {}:\n{}", cmd, output_str);
                    } else {
                        info!("  {} ran but produced no output", cmd);
                    }
                } else {
                    warn!("  Unable to run {}: may not have permissions or RDMA hardware", cmd);
                }
            }
        }
    }

    Ok(())
}

async fn check_rdma_prerequisites_linux() -> Result<()> {
    info!("Checking for RDMA prerequisites on Linux...");
    
    // Check for RDMA packages
    let packages_to_check = [
        "libibverbs-dev",
        "librdmacm-dev",
        "rdma-core",
    ];
    
    let package_managers = [
        ("dpkg -l", "debian"),
        ("rpm -qa", "redhat"),
        ("pacman -Q", "arch"),
    ];
    
    let mut found_packages = false;
    
    for (pm_cmd, pm_name) in &package_managers {
        let pm_check = Command::new("which")
            .arg(pm_cmd.split_whitespace().next().unwrap())
            .output();
        
        if let Ok(output) = pm_check {
            if output.status.success() {
                info!("Checking for RDMA packages using {} package manager...", pm_name);
                
                for package in &packages_to_check {
                    let check_cmd = format!("{} | grep {}", pm_cmd, package);
                    
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&check_cmd)
                        .output();
                    
                    if let Ok(output) = output {
                        if output.status.success() && !output.stdout.is_empty() {
                            info!("✅ Found RDMA package: {}", package);
                            found_packages = true;
                        }
                    }
                }
            }
        }
    }
    
    if !found_packages {
        warn!("❌ No RDMA packages found");
        info!("  Consider installing RDMA packages:");
        info!("    - On Debian/Ubuntu: sudo apt install rdma-core libibverbs-dev librdmacm-dev");
        info!("    - On RedHat/Fedora: sudo dnf install rdma-core libibverbs-devel librdmacm-devel");
        info!("    - On Arch Linux: sudo pacman -S rdma-core");
    }
    
    // Check for RDMA-capable network interfaces
    info!("Checking for RDMA-capable network interfaces...");
    
    let rdma_interfaces_cmd = Command::new("sh")
        .arg("-c")
        .arg("ls -l /sys/class/infiniband/ 2>/dev/null")
        .output();
    
    if let Ok(output) = rdma_interfaces_cmd {
        if output.status.success() && !output.stdout.is_empty() {
            let interfaces = String::from_utf8_lossy(&output.stdout);
            info!("✅ Found RDMA-capable interfaces:");
            for line in interfaces.lines() {
                info!("  {}", line);
            }
        } else {
            warn!("❌ No RDMA-capable interfaces found in /sys/class/infiniband/");
        }
    } else {
        warn!("❌ Could not check for RDMA-capable interfaces");
    }
    
    // Check if RDMA modules are loaded
    info!("Checking for RDMA kernel modules...");
    
    let rdma_modules_cmd = Command::new("sh")
        .arg("-c")
        .arg("lsmod | grep -E 'ib_|rdma|mlx'")
        .output();
    
    if let Ok(output) = rdma_modules_cmd {
        if output.status.success() && !output.stdout.is_empty() {
            let modules = String::from_utf8_lossy(&output.stdout);
            info!("✅ Found RDMA-related kernel modules:");
            for line in modules.lines() {
                info!("  {}", line);
            }
        } else {
            warn!("❌ No RDMA-related kernel modules found");
        }
    } else {
        warn!("❌ Could not check for RDMA kernel modules");
    }

    Ok(())
}

async fn detect_rdma_devices() -> Result<()> {
    // First try using 'ibv_devices' command if it exists
    let ibv_devices_cmd = Command::new("sh")
        .arg("-c")
        .arg("which ibv_devices && ibv_devices")
        .output();
    
    if let Ok(output) = ibv_devices_cmd {
        if output.status.success() && !output.stdout.is_empty() {
            let devices = String::from_utf8_lossy(&output.stdout);
            info!("✅ RDMA devices detected via ibv_devices:");
            for line in devices.lines() {
                info!("  {}", line);
            }
            return Ok(());
        }
    }
    
    // Try to derive RDMA capability from other system information
    
    // On Linux, check if InfiniBand or RoCE-capable devices exist
    if std::env::consts::OS == "linux" {
        // Check for Mellanox/NVIDIA NICs (common RDMA-capable devices)
        let lspci_cmd = Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -i 'mellanox\\|infiniband\\|roce'")
            .output();
        
        if let Ok(output) = lspci_cmd {
            if output.status.success() && !output.stdout.is_empty() {
                let devices = String::from_utf8_lossy(&output.stdout);
                info!("✅ Potentially RDMA-capable hardware found:");
                for line in devices.lines() {
                    info!("  {}", line);
                }
                return Ok(());
            }
        }
    }
    
    // On macOS, check PCIe devices
    if std::env::consts::OS == "macos" {
        let ioreg_cmd = Command::new("sh")
            .arg("-c")
            .arg("ioreg -l | grep -i 'mellanox\\|infiniband\\|thunderbolt'")
            .output();
        
        if let Ok(output) = ioreg_cmd {
            if output.status.success() && !output.stdout.is_empty() {
                let devices = String::from_utf8_lossy(&output.stdout);
                info!("✅ Potentially RDMA-capable hardware found:");
                for line in devices.lines() {
                    info!("  {}", line);
                }
                return Ok(());
            }
        }
        
        // Check Network interfaces
        let networksetup_cmd = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output();
        
        if let Ok(output) = networksetup_cmd {
            if output.status.success() {
                let interfaces = String::from_utf8_lossy(&output.stdout);
                info!("Available network interfaces:");
                for line in interfaces.lines() {
                    info!("  {}", line);
                }
                
                // Note: We can't definitively determine RDMA capability from interface list alone
                info!("  Note: Cannot determine RDMA capability from interface list alone");
            }
        }
    }
    
    warn!("❌ No RDMA devices detected");
    Ok(())
}

async fn assess_rdma_support() -> Result<()> {
    // Based on our checks, make a determination
    let os_type = std::env::consts::OS;
    
    match os_type {
        "macos" => {
            // Check for any RDMA-related libraries as a last resort
            let find_cmd = Command::new("sh")
                .arg("-c")
                .arg("find /usr/local/lib /opt/homebrew/lib /usr/lib -name \"*rdma*\" -o -name \"*verbs*\" 2>/dev/null")
                .output();
            
            let mut has_any_rdma_components = false;
            
            if let Ok(output) = find_cmd {
                if output.status.success() && !output.stdout.is_empty() {
                    has_any_rdma_components = true;
                }
            }
            
            if has_any_rdma_components {
                info!("⚠️ Some RDMA components were detected, but full RDMA support on macOS is UNLIKELY");
                info!("  • While some libraries were found, macOS lacks official RDMA drivers");
                info!("  • Thunderbolt might provide the hardware capability, but software support is missing");
            } else {
                info!("❌ RDMA is NOT SUPPORTED on macOS/Apple Silicon at this time");
                info!("  • No standard RDMA libraries are available for macOS");
                info!("  • Even with Thunderbolt 5's high bandwidth, macOS lacks RDMA drivers");
                info!("  • While Thunderbolt uses PCIe, which could theoretically support RDMA,");
                info!("    there is no evidence of RDMA capability in Apple's Thunderbolt implementation");
            }
            
            info!("\nRecommendation:");
            info!("  → Use the optimized TCP file transfer system for high-throughput transfers");
            info!("  → Run the 'test_file_transfer.sh' script to test TCP-based transfers");
        },
        "linux" => {
            // Check if we found RDMA devices earlier
            let ibv_devices_cmd = Command::new("sh")
                .arg("-c")
                .arg("which ibv_devices && ibv_devices")
                .output();
            
            if let Ok(output) = ibv_devices_cmd {
                if output.status.success() && !output.stdout.is_empty() {
                    info!("✅ RDMA appears to be SUPPORTED on this Linux system");
                    info!("  • RDMA libraries and tools are installed");
                    info!("  • RDMA-capable devices were detected");
                    
                    info!("\nRecommendation:");
                    info!("  → Proceed with RDMA implementation for maximum throughput");
                } else {
                    info!("⚠️ RDMA support is PARTIAL on this Linux system");
                    info!("  • Some RDMA components may be installed");
                    info!("  • However, no RDMA devices were detected");
                    
                    info!("\nRecommendation:");
                    info!("  → Install necessary RDMA hardware or drivers");
                    info!("  → Or use the optimized TCP file transfer system as a fallback");
                }
            } else {
                info!("❌ RDMA is NOT CONFIGURED on this Linux system");
                info!("  • RDMA libraries and tools are not installed or not in PATH");
                
                info!("\nRecommendation:");
                info!("  → Install RDMA packages (see earlier messages for instructions)");
                info!("  → Or use the optimized TCP file transfer system as a fallback");
            }
        },
        _ => {
            info!("⚠️ RDMA support on {} is UNDETERMINED", os_type);
            info!("  • This operating system is not commonly used with RDMA");
            
            info!("\nRecommendation:");
            info!("  → Use the optimized TCP file transfer system for file transfers");
        }
    }
    
    // Check if the TCP file transfer is already set up
    let file_transfer_path = std::path::Path::new("src/networking/file_transfer.rs");
    if file_transfer_path.exists() {
        info!("\nTCP fallback:");
        info!("  ✅ Optimized TCP file transfer system is available");
        info!("  → Use ./test_file_transfer.sh to run the file transfer utility");
    } else {
        info!("\nTCP fallback:");
        info!("  ❌ Optimized TCP file transfer system is not yet set up");
        info!("  → Implement the file_transfer.rs module for high-performance transfers");
    }
    
    Ok(())
} 
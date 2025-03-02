# RDMA Testing for Thunderbolt 5 on Apple Silicon

This utility tests for RDMA (Remote Direct Memory Access) capabilities on Apple Silicon systems, especially those with Thunderbolt 5 interfaces.

## Background

RDMA allows for zero-copy data transfers directly between application memory on different systems without CPU involvement, offering:
- Near-wire speed performance
- Minimal CPU overhead
- Zero-copy transfers
- Low latency

Thunderbolt 5 offers up to 80 Gbps bandwidth, making it a potential candidate for RDMA technology, although official RDMA support on Apple Silicon is not documented.

## What This Test Does

The `test_rdma` utility performs the following checks:

1. **RDMA Device Detection**: Identifies if any RDMA-capable devices are present on the system
2. **Capability Inspection**: For detected devices, reports capabilities such as queue pairs, completion queues, etc.
3. **Memory Registration Test**: Attempts to register memory for RDMA operations
4. **System Information**: Reports detailed system info including OS version, CPU, and Thunderbolt version
5. **Summary Assessment**: Provides a final assessment of RDMA support level

## Running the Test

To run the test:

```bash
./test_rdma.sh [log_level]
```

Where `log_level` is optional (defaults to "info") and can be:
- `trace`: Most verbose output
- `debug`: Detailed debugging information
- `info`: General information (default)
- `warn`: Warnings only
- `error`: Errors only

## Interpreting Results

The test will provide one of these conclusions:

- **FULLY SUPPORTED**: RDMA capabilities are detected and functional
- **LIMITED SUPPORT**: RDMA devices are detected but full functionality is limited
- **NOT SUPPORTED**: No RDMA capabilities detected

## Note on Apple Silicon Support

As of the latest macOS versions, official RDMA support for Apple Silicon is not documented. This utility is exploratory in nature and helps determine if any RDMA capabilities might be accessible.

If RDMA is not supported, the utility will recommend using optimized TCP for high-throughput transfers instead.

## Known Limitations

1. RDMA traditionally requires specific hardware and driver support
2. Apple's Thunderbolt implementations may not expose RDMA interfaces
3. Even if detected, full RDMA functionality may be limited

## Dependencies

This utility requires:
- Rust toolchain
- RDMA system libraries (if available)
- pkg-config (installed automatically if using the script)

## For High-Speed File Transfers

If RDMA is not supported, consider implementing:
1. Custom TCP implementations with zero-copy optimizations
2. io_uring (on Linux systems)
3. Kernel bypass techniques where available 
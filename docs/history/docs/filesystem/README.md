# Filesystem Documentation

This document outlines the filesystem strategy for Theseus OS.

## Goals

*   Provide robust, reliable, and performant filesystem options.
*   Support modern features like snapshots, compression, and encryption.
*   Allow flexibility based on device characteristics and user needs.

## Chosen Filesystems

1.  **BTRFS**:
    *   **Pros**: Snapshots, checksums, transparent compression, CoW, RAID capabilities. Ideal for general-purpose use and system resilience.
    *   **Use Case**: Default filesystem for most installations, especially for systems requiring data integrity and snapshotting capabilities.

2.  **XFS**:
    *   **Pros**: High performance, especially for large files and concurrent I/O. Mature and stable.
    *   **Use Case**: High-performance scenarios, servers, or applications where raw speed is prioritized over features like snapshots.

3.  **F2FS**:
    *   **Pros**: Optimized for flash-based storage (eMMC, SD cards), reducing write amplification and improving lifespan.
    *   **Use Case**: Primary filesystem for mobile devices and embedded systems with flash storage.

## Other Considerations

*   **Encryption**: Support for full-disk encryption and per-directory encryption will be provided at the kernel level.
*   **Mount Options**: Default mount options will be optimized for each filesystem type and intended use case.

## Status: Proposed / Research Needed / Implemented

## Design Decisions

*   [Link to relevant ADRs if any]

## Future Work

*   Evaluate performance benchmarks for each filesystem on target hardware.
*   Define default mount options and tuning parameters.
RNN: This is a good starting point. I recommend making BTRFS the default for most use cases due to its advanced features like snapshots, which align well with the "Ship of Theseus" philosophy of preserving state.

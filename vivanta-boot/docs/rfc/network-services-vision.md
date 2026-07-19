# RFC (Vision): Network Services Architecture and Reticulum Integration

## Status

Vision (Deferred)

Blocked by:

- Stage 6 — Userspace
- Stage 7 — Native Service Framework
- Stage 8 — Device Architecture
- Basic networking subsystem
- Driver framework for network interfaces

---

# Motivation

Vivanta aims to become a distributed operating system rather than simply another Unix-compatible kernel.

One of the long-term goals is allowing multiple Vivanta devices to communicate securely over heterogeneous physical transports without requiring applications to understand individual networking technologies.

Reticulum represents an interesting candidate for this because it provides:

- cryptographic identities
- transport independence
- mesh networking
- multi-hop routing
- automatic path discovery
- encrypted communication
- operation over unreliable links

These properties align naturally with the long-term goals of Vivanta.

However, Reticulum must not become part of the kernel.

---

# Architectural Principle

Reticulum is **not** a device driver.

Reticulum is **not** a transport.

Reticulum is **not** a kernel subsystem.

Reticulum is a **Network Service**.

The kernel exposes hardware.

Network services implement protocols.

---

# Layered Architecture

```
Applications
        |
Vivanta SDK
        |
Native Vivanta API
        |
----------------------------------
Network Service Framework
----------------------------------
Reticulum Service
TCP/IP Service
Bluetooth Mesh Service
Matter Service
Future Protocols...
----------------------------------
Kernel Network Interface API
----------------------------------
Ethernet Driver
Wi-Fi Driver
LoRa Driver
LTE Driver
Serial Driver
USB CDC Driver
CAN Driver
```

The kernel should never know which network protocol is running.

The kernel only provides packet transmission and reception.

---

# Kernel Responsibilities

The kernel is responsible only for:

- network device discovery
- packet transmit
- packet receive
- interrupt delivery
- DMA management
- network buffers
- interface statistics
- power management

The kernel should never perform:

- routing
- encryption
- peer discovery
- protocol parsing
- mesh management

Those belong to user-space services.

---

# Network Interface API

Every hardware driver exposes a minimal interface.

Example:

```
NetworkInterface

name()
mtu()
transmit(packet)
receive()
link_state()
statistics()
```

No protocol-specific functionality exists here.

The interface must work equally for:

- Ethernet
- Wi-Fi
- LTE
- LoRa
- Serial
- USB
- Virtual interfaces

---

# Network Service Framework

Userspace services implement protocols.

Each service communicates only through the generic interface.

Example:

```
NetworkService

start()
stop()
enumerate_interfaces()
send()
receive()
```

The framework itself knows nothing about Reticulum.

Reticulum is simply one implementation.

---

# Reticulum Service

Reticulum becomes a normal userspace daemon.

Responsibilities:

- destination management
- routing
- path discovery
- packet encryption
- packet fragmentation
- transport selection
- peer discovery
- mesh operation

Kernel involvement is zero.

---

# Transport Independence

Applications must never know which physical transport is currently used.

Example:

```
Application
      |
Reticulum
      |
Destination
      |
-----------------------------
Wi-Fi

or

LoRa

or

LTE

or

Ethernet

or

USB

or

Serial
```

The transport may change dynamically.

Applications continue communicating with the same destination.

---

# Relationship with Identity

RFC-001 introduces persistent cryptographic identity.

Long-term integration could look like:

```
Identity
        |
Public Key
        |
Reticulum Destination
        |
Encrypted Link
```

This creates a natural mapping between system identity and network identity.

No additional addressing model is required.

---

# Coexistence with TCP/IP

Reticulum does not replace TCP/IP.

Both stacks may exist simultaneously.

Example:

```
Native Vivanta App
        |
Reticulum

Linux-compatible App
        |
POSIX
        |
TCP/IP
```

The operating system supports multiple networking models.

Applications choose whichever is appropriate.

---

# Future Network Services

The architecture must remain protocol-neutral.

Possible future implementations:

- Reticulum
- TCP/IP
- Bluetooth Mesh
- Matter
- CAN Open
- DDS
- Custom industrial protocols

No protocol receives privileged treatment.

---

# Why Userspace?

Keeping Reticulum outside the kernel provides:

- independent updates
- smaller trusted computing base
- easier debugging
- protocol evolution
- lower maintenance burden
- reduced kernel complexity

The kernel remains transport-oriented.

Protocol evolution happens entirely in userspace.

---

# Design Rules

1. No protocol-specific code inside the kernel.
2. Kernel owns hardware.
3. Userspace owns routing.
4. Userspace owns cryptography.
5. Applications communicate with services, not interfaces.
6. Physical transport must remain replaceable.
7. Multiple protocol stacks may coexist.
8. Identity integration must remain optional.
9. The framework must never assume Reticulum is the default protocol.

---

# Success Criteria

The architecture is considered successful if the same application can communicate with another Vivanta node while the underlying transport changes transparently between:

- Ethernet
- Wi-Fi
- LTE
- LoRa
- Serial
- USB

without modifying application code.

---

# Deferred Until

Implementation must not begin before:

- stable scheduler
- userspace
- ELF loader
- system services
- driver framework
- network device API

Expected implementation horizon:

After Stage 8 of the Architecture Roadmap.

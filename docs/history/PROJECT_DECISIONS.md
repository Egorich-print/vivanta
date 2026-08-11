# Accepted Decisions

This document serves as a journal of architectural decisions made for the Vivanta project. It records the decision, its status, the rationale behind it, alternatives considered, and the source of the decision.

---

## Decision: Project Identity Definition

*   **Decision**: Redefine project identity from "Operating System" to "**Adaptive Computing Platform**" or "**Adaptive Operating Platform**". The OS is a component within this broader platform.
*   **Status**: Accepted
*   **Rationale**: To better reflect the project's core goals of adaptability, hardware evolution, and user environment preservation, which extend beyond traditional OS boundaries. Emphasizes the "Adaptive Engine" as a central concept.
*   **Alternatives Considered**: Remaining strictly an "Operating System."
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Foundational Principles

*   **Decision**: Adopt and adhere to a set of core project principles: User First, Adaptive System, Architecture Independence, Minimize Manual Configuration, Document Before Code, Modularity and Composability.
*   **Status**: Accepted
*   **Rationale**: These principles provide a guiding framework for all architectural and implementation decisions, ensuring long-term vision alignment.
*   **Alternatives Considered**: Less formalized principles, context-dependent decision-making.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Core Invariants

*   **Decision**: Establish core invariants that must be maintained throughout the project's lifecycle: User Environment Preservation, Hardware Adaptability, Architecture Independence, Minimal Friction, Documentation as Source of Truth, Long-Term Maintainability.
*   **Status**: Accepted
*   **Rationale**: These invariants define the fundamental requirements that the system must fulfill, acting as non-negotiable constraints.
*   **Alternatives Considered**: Relying solely on principles without formalizing them as invariants.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Primary Systems Programming Language

*   **Decision**: Designate **Rust** as the primary systems programming language for Vivanta.
*   **Status**: Accepted
*   **Rationale**: Chosen for its guarantees in memory safety, concurrency, and long-term maintainability, aligning with the project's goals. This is the primary language, allowing for pragmatic exceptions if architecturally justified, rather than an immutable constraint.
*   **Alternatives Considered**: C, C++, Go, Zig. Rust was selected for its balance of safety, performance, and modern features suitable for systems programming.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Initial Hardware Target

*   **Decision**: Select **Xiaomi Redmi Note 7 (lavender)** as the initial hardware target for Milestone 1.
*   **Status**: Accepted
*   **Rationale**: Chosen for its excellent Linux support, active community, and the benefit of forcing optimization on older hardware, aligning with the goal of portability and optimization.
*   **Alternatives Considered**: Other development boards or generic x86 hardware. The specific choice was made for its rich existing documentation and community support relevant to adaptation.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Development Process & Documentation

*   **Decision**: Adopt a phased development approach starting with R0 Peer Review, followed by architecture definition, and then repository creation. Emphasize "Document Before Code."
*   **Status**: Accepted
*   **Rationale**: Ensures a strong architectural foundation is established before implementation, preventing costly rework and promoting long-term viability.
*   **Alternatives Considered**: Begin implementation directly, iterative design alongside coding.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Documentation Artifacts

*   **Decision**: Utilize core state files (`STATUS.md`, `PROJECT_DECISIONS.md`, `OPEN_QUESTIONS.md`, `PROJECT_GLOSSARY.md`) as living architectural artifacts, serving as the primary source of truth and context.
*   **Status**: Accepted
*   **Rationale**: Provides a concise, efficient, and maintainable way to manage project knowledge across sessions and over the project's lifetime.
*   **Alternatives Considered**: Relying solely on conversation history, a single monolithic state file.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

## Decision: Naming Conventions & Structure

*   **Decision**: Adopt specific directory naming conventions (`specs/`, `concepts/`, `research/` (top-level), `decisions/`, `core/`, `milestones/`, etc.) and file naming (`R0_PEER_REVIEW.md`, `MANIFESTO.md`, `VISION.md`, `THESEUS_BOOK.md`).
*   **Status**: Accepted
*   **Rationale**: Creates a logical, scalable, and professional project structure that facilitates navigation and understanding.
*   **Alternatives Considered**: Variations in directory organization, different naming schemes.
*   **Date**: 2026-07-10
*   **Source**: Conversation Log / User Direction

---

**Note:** This journal will be updated as new architectural decisions are made and accepted.

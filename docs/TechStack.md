# TechStack.md

## Technology Reference

| Component | Technology |
|---|---|
| **Language** | Rust (stable) |
| **GPU Backend** | `wgpu` (Vulkan/DX12/Metal/WebGPU) |
| **Windowing** | `winit` |
| **UI Framework** | `egui` |
| **Async Runtime** | `tokio` |
| **HTTP Client** | `reqwest` |
| **UDP socket** | `std::net::UdpSocket` |
| **Serialization** | `serde` (JSON, LLSD, XML-RPC) |
| **Telemetery** | `tracing` |

## Code Architecture

Built as a multi-crate Cargo workspace with strict boundaries.

- `viewer_app`: Orchestration and Frame Loop
- `viewer_core`: Shared Domain State
- `viewer_render`: wgpu Renderer
- `viewer_ui`: egui Panels and Overlay
- `viewer_net`: Network transport and session diagnostics
- `viewer_grid`: Grid request/response semantics
- `viewer_asset`: (Reserved) Asset lifecycle
- `viewer_platform`: (Reserved) OS Integration

For detailed crate responsibilities and data flows, see `docs/ARCHITECTURE.md`.
For the current observable state, see `docs/CURRENT_STATE.md`.

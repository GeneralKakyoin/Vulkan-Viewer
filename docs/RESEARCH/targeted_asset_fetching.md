# RESEARCH: Targeted Asset Fetching ("Single Item" Bridge)

## Question Investigated
Is it possible to resolve and texture a single specific mesh asset from the SL/OpenSim grid following the Milestone 4 (Performance) work, without implementing the full-scene ingestion background worker?

## Behavior Summary (Firestorm Reference)
- **Capability**: The `ViewerAsset` CAP (found in `llviewerassetstorage.cpp`) is the primary endpoint for fetching binary mesh data.
- **Request Format**: A standard HTTP GET request with a query parameter: `?mesh_id=<uuid>`.
- **Response Format**: 
    - **Legacy Mesh**: A custom binary LLSD format containing vertex/index blocks.
    - **PBR Mesh**: A GLB/glTF binary blob (Standard).
- **Texturing**: Textures are fetched separately via the `GetTexture` CAP using the UUIDs refined in the mesh's material descriptors.

## Crate Boundary Mapping
- **`viewer_net`**: Should provide a `fetch_single_asset(cap_url, uuid)` helper.
- **`viewer_asset`**: Keeps the `GeometryCache` and `TextureStore`. The `mesh_loader` already has a `load_gltf_mesh` function that is compatible with modern PBR assets.
- **`viewer_app`**: Orchestrates a "Targeted Fetch" when a specific `VIEWER_TARGET_UUID` environment variable is set.

## Conclusion
Yes, this is highly feasible after Milestone 4. By deferring it until after M4, we ensure that:
1. The **VRAM Budgeter** can handle the incoming mesh size safely.
2. The **Draw Pools** can correctly categorize the new object.
3. The **Occlusion Culling** works on the new object immediately.

A "Single Item" test is recommended over full background ingestion to validate the transport layer without complexity.

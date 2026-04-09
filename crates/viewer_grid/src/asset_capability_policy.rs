pub struct AssetCapabilityPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityProbeMethod {
    Get,
    Post,
    Del,
}

impl CapabilityProbeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Del => "DEL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerAssetQueryKey {
    TextureId,
    MeshId,
    MaterialId,
    AnimatnId,
    SoundId,
}

impl ViewerAssetQueryKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextureId => "texture_id",
            Self::MeshId => "mesh_id",
            Self::MaterialId => "material_id",
            Self::AnimatnId => "animatn_id",
            Self::SoundId => "sound_id",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProbeRequestShape {
    pub capability_name: String,
    pub method: CapabilityProbeMethod,
    pub url_candidates: Vec<String>,
    pub accept: Option<String>,
    pub content_type: Option<String>,
    pub body: Option<String>,
    pub query_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshCapabilityRequestCandidate {
    pub capability_name: String,
    pub url_variant: String,
    pub url: String,
}

impl AssetCapabilityPolicy {
    pub fn texture_url_candidates(
        capabilities: &std::collections::BTreeMap<String, String>,
        asset_id: &viewer_core::AssetID,
    ) -> Vec<String> {
        if asset_id.is_empty() {
            return Vec::new();
        }

        let mut urls = Vec::new();
        if let Some(base_url) = capabilities.get("GetTexture") {
            extend_unique(
                &mut urls,
                Self::texture_url_candidates_from_base(base_url, asset_id.as_str()),
            );
        }
        if let Some(base_url) = capabilities.get("ViewerAsset") {
            extend_unique(
                &mut urls,
                Self::texture_url_candidates_from_base(base_url, asset_id.as_str()),
            );
        }
        urls
    }

    pub fn texture_url_candidates_from_base(base_url: &str, asset_id: &str) -> Vec<String> {
        let base = base_url.trim().trim_end_matches('/');
        let asset_id = asset_id.trim();
        if base.is_empty() || asset_id.is_empty() {
            return Vec::new();
        }

        let mut urls = Vec::new();
        extend_unique(
            &mut urls,
            [
                format!("{base}/?texture_id={asset_id}"),
                format!("{base}?texture_id={asset_id}"),
                format!("{base}/{asset_id}"),
                format!("{base}?id={asset_id}"),
                format!("{base}?asset_id={asset_id}"),
            ],
        );
        urls
    }

    pub fn viewer_asset_url_candidates_from_base(
        base_url: &str,
        query_key: ViewerAssetQueryKey,
        asset_id: &str,
    ) -> Vec<String> {
        let base = base_url.trim().trim_end_matches('/');
        let asset_id = asset_id.trim();
        if base.is_empty() || asset_id.is_empty() {
            return Vec::new();
        }
        let key = query_key.as_str();
        let mut urls = Vec::new();
        extend_unique(
            &mut urls,
            [
                format!("{base}/?{key}={asset_id}"),
                format!("{base}?{key}={asset_id}"),
                format!("{base}/{asset_id}"),
                format!("{base}?id={asset_id}"),
                format!("{base}?asset_id={asset_id}"),
            ],
        );
        urls
    }

    pub fn mesh_request_candidates(
        capabilities: &std::collections::BTreeMap<String, String>,
        mesh_id: &str,
    ) -> Vec<MeshCapabilityRequestCandidate> {
        let mesh_id = mesh_id.trim();
        if mesh_id.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        // Firestorm mesh cap preference: ViewerAsset, then GetMesh2, then GetMesh.
        for cap_name in ["ViewerAsset", "GetMesh2", "GetMesh"] {
            let Some(base) = capabilities.get(cap_name) else {
                continue;
            };
            let base = base.trim().trim_end_matches('/');
            if base.is_empty() {
                continue;
            }
            extend_mesh_request_candidates(
                &mut candidates,
                cap_name,
                [
                    ("v1", format!("{base}/?mesh_id={mesh_id}")),
                    ("v2", format!("{base}?mesh_id={mesh_id}")),
                ],
            );
        }
        candidates
    }

    pub fn mesh_url_candidates(
        capabilities: &std::collections::BTreeMap<String, String>,
        mesh_id: &str,
    ) -> Vec<String> {
        Self::mesh_request_candidates(capabilities, mesh_id)
            .into_iter()
            .map(|candidate| candidate.url)
            .collect()
    }

    pub fn simulator_features_probe_shape(
        capabilities: &std::collections::BTreeMap<String, String>,
    ) -> Option<CapabilityProbeRequestShape> {
        let url = capabilities.get("SimulatorFeatures")?.trim();
        if url.is_empty() {
            return None;
        }
        Some(CapabilityProbeRequestShape {
            capability_name: String::from("SimulatorFeatures"),
            method: CapabilityProbeMethod::Get,
            url_candidates: vec![url.to_string()],
            accept: None,
            content_type: None,
            body: None,
            query_key: None,
        })
    }

    pub fn interest_list_probe_shape(
        capabilities: &std::collections::BTreeMap<String, String>,
        mode: &str,
    ) -> Option<CapabilityProbeRequestShape> {
        let url = capabilities.get("InterestList")?.trim();
        if url.is_empty() {
            return None;
        }
        let mode = if mode.trim().is_empty() {
            "default"
        } else {
            mode.trim()
        };
        let body = format!("<llsd><map><key>mode</key><string>{mode}</string></map></llsd>");
        Some(CapabilityProbeRequestShape {
            capability_name: String::from("InterestList"),
            method: CapabilityProbeMethod::Post,
            url_candidates: vec![url.to_string()],
            accept: Some(String::from("application/llsd+xml")),
            content_type: Some(String::from("application/llsd+xml")),
            body: Some(body),
            query_key: None,
        })
    }

    pub fn untrusted_simulator_message_probe_shape(
        capabilities: &std::collections::BTreeMap<String, String>,
    ) -> Option<CapabilityProbeRequestShape> {
        let url = capabilities.get("UntrustedSimulatorMessage")?.trim();
        if url.is_empty() {
            return None;
        }
        Some(CapabilityProbeRequestShape {
            capability_name: String::from("UntrustedSimulatorMessage"),
            method: CapabilityProbeMethod::Get,
            url_candidates: vec![url.to_string()],
            accept: Some(String::from("application/llsd+xml")),
            content_type: None,
            body: None,
            query_key: None,
        })
    }

    pub fn viewer_asset_probe_shape_from_base(
        viewer_asset_base_url: &str,
        query_key: ViewerAssetQueryKey,
        asset_id: &str,
    ) -> Option<CapabilityProbeRequestShape> {
        let urls =
            Self::viewer_asset_url_candidates_from_base(viewer_asset_base_url, query_key, asset_id);
        if urls.is_empty() {
            return None;
        }
        Some(CapabilityProbeRequestShape {
            capability_name: String::from("ViewerAsset"),
            method: CapabilityProbeMethod::Get,
            url_candidates: urls,
            accept: Some(String::from("*/*")),
            content_type: None,
            body: None,
            query_key: Some(String::from(query_key.as_str())),
        })
    }

    pub fn select_texture_url(
        capabilities: &std::collections::BTreeMap<String, String>,
        asset_id: &viewer_core::AssetID,
    ) -> Option<String> {
        Self::texture_url_candidates(capabilities, asset_id)
            .into_iter()
            .next()
    }
}

pub(crate) fn extend_unique<I>(out: &mut Vec<String>, candidates: I)
where
    I: IntoIterator<Item = String>,
{
    for candidate in candidates {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
}

pub(crate) fn extend_mesh_request_candidates<I>(
    out: &mut Vec<MeshCapabilityRequestCandidate>,
    capability_name: &str,
    candidates: I,
) where
    I: IntoIterator<Item = (&'static str, String)>,
{
    for (url_variant, url) in candidates {
        if !out.iter().any(|candidate| candidate.url == url) {
            out.push(MeshCapabilityRequestCandidate {
                capability_name: capability_name.to_string(),
                url_variant: url_variant.to_string(),
                url,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_policy_prioritizes_get_texture() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetTexture".to_string(), "http://cdn".to_string());
        caps.insert("ViewerAsset".to_string(), "http://fallback".to_string());

        let id = viewer_core::AssetID::new("test-id");
        let url = AssetCapabilityPolicy::select_texture_url(&caps, &id).unwrap();
        assert_eq!(url, "http://cdn/?texture_id=test-id");
    }

    #[test]
    fn asset_policy_exposes_firestorm_style_first_then_legacy_fallbacks() {
        let candidates =
            AssetCapabilityPolicy::texture_url_candidates_from_base("http://cdn", "test-id");
        assert_eq!(
            candidates,
            vec![
                "http://cdn/?texture_id=test-id",
                "http://cdn?texture_id=test-id",
                "http://cdn/test-id",
                "http://cdn?id=test-id",
                "http://cdn?asset_id=test-id",
            ]
        );
    }

    #[test]
    fn asset_policy_falls_back_to_viewer_asset() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("ViewerAsset".to_string(), "http://fallback".to_string());

        let id = viewer_core::AssetID::new("test-id");
        let url = AssetCapabilityPolicy::select_texture_url(&caps, &id).unwrap();
        assert_eq!(url, "http://fallback/?texture_id=test-id");
    }

    #[test]
    fn asset_policy_returns_none_on_missing_caps_or_id() {
        let caps = std::collections::BTreeMap::new();
        let id = viewer_core::AssetID::new("test-id");
        assert!(AssetCapabilityPolicy::select_texture_url(&caps, &id).is_none());

        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetTexture".to_string(), "http://cdn".to_string());
        assert!(
            AssetCapabilityPolicy::select_texture_url(&caps, &viewer_core::AssetID::default())
                .is_none()
        );
        assert!(
            AssetCapabilityPolicy::texture_url_candidates(&caps, &viewer_core::AssetID::default())
                .is_empty()
        );
    }

    #[test]
    fn viewer_asset_probe_candidates_prefer_firestorm_query_shape() {
        let candidates = AssetCapabilityPolicy::viewer_asset_url_candidates_from_base(
            "http://cdn",
            ViewerAssetQueryKey::MeshId,
            "mesh-uuid",
        );
        assert_eq!(
            candidates,
            vec![
                "http://cdn/?mesh_id=mesh-uuid",
                "http://cdn?mesh_id=mesh-uuid",
                "http://cdn/mesh-uuid",
                "http://cdn?id=mesh-uuid",
                "http://cdn?asset_id=mesh-uuid",
            ]
        );
    }

    #[test]
    fn mesh_url_candidates_prefer_viewer_asset_then_getmesh2_then_getmesh() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetMesh".to_string(), "http://mesh1".to_string());
        caps.insert("GetMesh2".to_string(), "http://mesh2".to_string());
        caps.insert("ViewerAsset".to_string(), "http://asset-cdn".to_string());

        let candidates = AssetCapabilityPolicy::mesh_url_candidates(&caps, "mesh-uuid");
        assert_eq!(
            candidates,
            vec![
                "http://asset-cdn/?mesh_id=mesh-uuid",
                "http://asset-cdn?mesh_id=mesh-uuid",
                "http://mesh2/?mesh_id=mesh-uuid",
                "http://mesh2?mesh_id=mesh-uuid",
                "http://mesh1/?mesh_id=mesh-uuid",
                "http://mesh1?mesh_id=mesh-uuid",
            ]
        );
    }

    #[test]
    fn mesh_url_candidates_fall_back_to_legacy_caps() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetMesh2".to_string(), "http://mesh2".to_string());
        let candidates = AssetCapabilityPolicy::mesh_url_candidates(&caps, "mesh-uuid");
        assert_eq!(
            candidates,
            vec![
                "http://mesh2/?mesh_id=mesh-uuid",
                "http://mesh2?mesh_id=mesh-uuid",
            ]
        );
    }

    #[test]
    fn mesh_request_candidates_include_capability_metadata_in_priority_order() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert("GetMesh".to_string(), "http://mesh1".to_string());
        caps.insert("GetMesh2".to_string(), "http://mesh2".to_string());
        caps.insert("ViewerAsset".to_string(), "http://asset-cdn".to_string());

        let candidates = AssetCapabilityPolicy::mesh_request_candidates(&caps, "mesh-uuid");
        assert_eq!(
            candidates,
            vec![
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("ViewerAsset"),
                    url_variant: String::from("v1"),
                    url: String::from("http://asset-cdn/?mesh_id=mesh-uuid"),
                },
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("ViewerAsset"),
                    url_variant: String::from("v2"),
                    url: String::from("http://asset-cdn?mesh_id=mesh-uuid"),
                },
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("GetMesh2"),
                    url_variant: String::from("v1"),
                    url: String::from("http://mesh2/?mesh_id=mesh-uuid"),
                },
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("GetMesh2"),
                    url_variant: String::from("v2"),
                    url: String::from("http://mesh2?mesh_id=mesh-uuid"),
                },
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("GetMesh"),
                    url_variant: String::from("v1"),
                    url: String::from("http://mesh1/?mesh_id=mesh-uuid"),
                },
                MeshCapabilityRequestCandidate {
                    capability_name: String::from("GetMesh"),
                    url_variant: String::from("v2"),
                    url: String::from("http://mesh1?mesh_id=mesh-uuid"),
                },
            ]
        );
    }

    #[test]
    fn simulator_features_probe_shape_is_get_without_body() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert(
            "SimulatorFeatures".to_string(),
            "http://sim/cap/simfeatures".to_string(),
        );
        let shape = AssetCapabilityPolicy::simulator_features_probe_shape(&caps)
            .expect("shape should exist");
        assert_eq!(shape.method, CapabilityProbeMethod::Get);
        assert!(shape.body.is_none());
        assert_eq!(shape.url_candidates, vec!["http://sim/cap/simfeatures"]);
    }

    #[test]
    fn interest_list_probe_shape_is_post_with_default_mode() {
        let mut caps = std::collections::BTreeMap::new();
        caps.insert(
            "InterestList".to_string(),
            "http://sim/cap/interest".to_string(),
        );
        let shape = AssetCapabilityPolicy::interest_list_probe_shape(&caps, "")
            .expect("shape should exist");
        assert_eq!(shape.method, CapabilityProbeMethod::Post);
        assert_eq!(shape.content_type.as_deref(), Some("application/llsd+xml"));
        assert_eq!(shape.accept.as_deref(), Some("application/llsd+xml"));
        assert!(
            shape
                .body
                .as_deref()
                .is_some_and(|body| body.contains("<string>default</string>"))
        );
    }

    #[test]
    fn viewer_asset_probe_shape_carries_query_key_label() {
        let shape = AssetCapabilityPolicy::viewer_asset_probe_shape_from_base(
            "http://asset-cdn",
            ViewerAssetQueryKey::MaterialId,
            "material-uuid",
        )
        .expect("shape should exist");
        assert_eq!(shape.capability_name, "ViewerAsset");
        assert_eq!(shape.method, CapabilityProbeMethod::Get);
        assert_eq!(shape.query_key.as_deref(), Some("material_id"));
        assert_eq!(
            shape.url_candidates.first().map(String::as_str),
            Some("http://asset-cdn/?material_id=material-uuid")
        );
    }
}

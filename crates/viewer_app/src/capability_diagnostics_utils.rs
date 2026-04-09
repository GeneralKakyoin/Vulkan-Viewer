use super::*;

pub(super) fn push_protocol_event(events: &mut Vec<String>, entry: impl Into<String>) {
    const MAX_PROTOCOL_EVENTS: usize = 24;
    if events.len() >= MAX_PROTOCOL_EVENTS {
        events.remove(0);
    }
    events.push(entry.into());
}

pub(super) fn format_capability_url_family(family: CapabilityUrlFamily) -> &'static str {
    match family {
        CapabilityUrlFamily::SimulatorHost12043 => "simhost:12043",
        CapabilityUrlFamily::SimulatorHost12046 => "simhost:12046",
        CapabilityUrlFamily::AssetCdn => "asset-cdn",
        CapabilityUrlFamily::BakeTextureCdn => "bake-texture-cdn",
        CapabilityUrlFamily::MapCdn => "map-cdn",
        CapabilityUrlFamily::PhoenixViewer => "phoenixviewer",
        CapabilityUrlFamily::Analytics => "analytics",
        CapabilityUrlFamily::GenericWeb => "generic-web",
        CapabilityUrlFamily::Unknown => "unknown",
    }
}

pub(super) fn format_classified_url(url: &str) -> String {
    let classification = classify_capability_url(url);
    let host = classification.host.unwrap_or_else(|| String::from("?"));
    match classification.port {
        Some(port) => format!(
            "{}@{}:{}",
            format_capability_url_family(classification.family),
            host,
            port
        ),
        None => format!(
            "{}@{}",
            format_capability_url_family(classification.family),
            host
        ),
    }
}

pub(super) fn format_capability_host_family_tag(url: &str) -> String {
    let classification = classify_capability_url(url);
    let host_family = classification
        .host
        .as_deref()
        .and_then(|host| host.split('.').next())
        .unwrap_or("unknown-host");
    format!("host_family={host_family}")
}

pub(super) fn format_mesh_fetch_attempt_line(
    id: &str,
    lod: u32,
    candidate: Option<&viewer_grid::MeshCapabilityRequestCandidate>,
    attempt: &viewer_net::AssetFetchAttempt,
) -> String {
    let capability_name = candidate
        .map(|candidate| candidate.capability_name.as_str())
        .unwrap_or("unknown");
    let url_variant = candidate
        .map(|candidate| candidate.url_variant.as_str())
        .unwrap_or("unknown");
    let family = format_capability_url_family(classify_capability_url(&attempt.url).family);
    let range = attempt.range_header.as_deref().unwrap_or("none");
    let status = attempt
        .status
        .map(|status| status.to_string())
        .unwrap_or_else(|| String::from("transport_error"));
    let bucket = attempt
        .status
        .zip(attempt.response_body.as_deref())
        .and_then(|(status, body)| classify_mesh_http_status_bucket(status, body))
        .map(|bucket| format!(" bucket={bucket}"))
        .unwrap_or_default();
    let error = attempt
        .error
        .as_deref()
        .map(|error| format!(" error={error}"))
        .unwrap_or_default();
    format!(
        "attempt id={} lod={} cap={} family={} url_variant={} range={} status={}{}{}",
        id, lod, capability_name, family, url_variant, range, status, bucket, error
    )
}

pub(super) const OBJECT_INGRESS_BASELINE_CAP_NAMES: [&str; 4] = [
    "EventQueueGet",
    "InterestList",
    "RegionObjects",
    "UntrustedSimulatorMessage",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LaneProbeShapeTask {
    pub(super) host_family: String,
    pub(super) capability_name: String,
    pub(super) query_key: Option<String>,
    pub(super) url_variant: String,
    pub(super) request: CapabilityProbeRequest,
}

pub(super) fn extract_lane_probe_asset_ids(config: &InProcessLiveFeedConfig) -> Vec<String> {
    config.lane_probe_asset_ids.clone()
}

pub(super) fn capability_probe_method_to_string(method: CapabilityProbeMethod) -> String {
    method.as_str().to_string()
}

fn shape_to_lane_probe_tasks(
    shape: &CapabilityProbeRequestShape,
    max_url_variants: usize,
) -> Vec<LaneProbeShapeTask> {
    if shape.url_candidates.is_empty() {
        return Vec::new();
    }
    let limit = max_url_variants.max(1);
    shape
        .url_candidates
        .iter()
        .take(limit)
        .enumerate()
        .map(|(idx, url)| {
            let host_family = classify_capability_url(url)
                .host
                .as_deref()
                .and_then(|host| host.split('.').next())
                .unwrap_or("unknown-host")
                .to_string();
            LaneProbeShapeTask {
                host_family,
                capability_name: shape.capability_name.clone(),
                query_key: shape.query_key.clone(),
                url_variant: format!("v{}", idx + 1),
                request: CapabilityProbeRequest {
                    method: capability_probe_method_to_string(shape.method),
                    url_candidates: vec![url.clone()],
                    accept: shape.accept.clone(),
                    content_type: shape.content_type.clone(),
                    body: shape.body.clone(),
                    ..Default::default()
                },
            }
        })
        .collect()
}

pub(super) fn build_lane_probe_shape_matrix(
    entries: &BTreeMap<String, String>,
    config: &InProcessLiveFeedConfig,
) -> (Vec<LaneProbeShapeTask>, Vec<LaneProbeShapeTask>) {
    let mut immediate = Vec::new();
    let mut gated = Vec::new();

    if let Some(shape) = AssetCapabilityPolicy::simulator_features_probe_shape(entries) {
        gated.extend(shape_to_lane_probe_tasks(&shape, 1));
    }
    if let Some(shape) = AssetCapabilityPolicy::interest_list_probe_shape(entries, "default") {
        gated.extend(shape_to_lane_probe_tasks(&shape, 1));
    }
    if let Some(shape) = AssetCapabilityPolicy::untrusted_simulator_message_probe_shape(entries) {
        gated.extend(shape_to_lane_probe_tasks(&shape, 1));
    }

    let viewer_asset_base = entries.get("ViewerAsset").cloned();
    if let Some(base_url) = viewer_asset_base {
        let asset_ids = extract_lane_probe_asset_ids(config);
        let mut probe_keys = vec![
            (
                ViewerAssetQueryKey::TextureId,
                asset_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| String::from("00000000-0000-0000-0000-000000000001")),
            ),
            (
                ViewerAssetQueryKey::SoundId,
                asset_ids
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| String::from("00000000-0000-0000-0000-000000000002")),
            ),
        ];
        let optional_keys = [
            ViewerAssetQueryKey::MeshId,
            ViewerAssetQueryKey::MaterialId,
            ViewerAssetQueryKey::AnimatnId,
        ];
        for (idx, key) in optional_keys.iter().enumerate() {
            if let Some(asset_id) = asset_ids.get(idx + 2) {
                probe_keys.push((*key, asset_id.clone()));
            }
        }
        for (key, asset_id) in probe_keys {
            if let Some(shape) =
                AssetCapabilityPolicy::viewer_asset_probe_shape_from_base(&base_url, key, &asset_id)
            {
                immediate.extend(shape_to_lane_probe_tasks(&shape, 2));
            }
        }
    }

    (immediate, gated)
}

pub(super) async fn run_lane_probe_shape_tasks_once(
    connection: &mut Connection,
    tx: &mpsc::Sender<LiveFeedUpdate>,
    protocol_events: &mut Vec<String>,
    tasks: &mut Vec<LaneProbeShapeTask>,
) {
    if tasks.is_empty() {
        return;
    }
    let draining = std::mem::take(tasks);
    for task in draining {
        let classified_url = task
            .request
            .url_candidates
            .first()
            .map(|url| format_classified_url(url))
            .unwrap_or_else(|| String::from("url=missing"));
        let query_key = task.query_key.as_deref().unwrap_or("none");
        push_protocol_event(
            protocol_events,
            format!(
                "LaneProbeShape:start host_family={} cap={} method={} query_key={} url_variant={} {}",
                task.host_family,
                task.capability_name,
                task.request.method,
                query_key,
                task.url_variant,
                classified_url
            ),
        );
        match connection
            .execute_capability_probe_once(&task.request)
            .await
        {
            Ok(probe) => {
                let content_type = probe.content_type.as_deref().unwrap_or("unknown");
                let preview_hash = probe.body_preview_hash.as_deref().unwrap_or("none");
                push_protocol_event(
                    protocol_events,
                    format!(
                        "LaneProbeShape:ok host_family={} cap={} method={} query_key={} url_variant={} status={} decode={} content_type={} body_bytes={} preview_hash={}",
                        task.host_family,
                        task.capability_name,
                        task.request.method,
                        query_key,
                        task.url_variant,
                        probe.status,
                        probe.decode,
                        content_type,
                        probe.body_bytes,
                        preview_hash,
                    ),
                );
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Info,
                    "parallel_protocol",
                    &format!(
                        "lane probe shape ok host_family={} cap={} method={} query_key={} url_variant={} status={} decode={} class={} content_type={} body_bytes={} preview_hash={} {}",
                        task.host_family,
                        task.capability_name,
                        task.request.method,
                        query_key,
                        task.url_variant,
                        probe.status,
                        probe.decode,
                        probe.response_class,
                        content_type,
                        probe.body_bytes,
                        preview_hash,
                        classified_url
                    ),
                );
            }
            Err(err) => {
                push_protocol_event(
                    protocol_events,
                    format!(
                        "LaneProbeShape:err host_family={} cap={} method={} query_key={} url_variant={} {}",
                        task.host_family,
                        task.capability_name,
                        task.request.method,
                        query_key,
                        task.url_variant,
                        err
                    ),
                );
                emit_relay(
                    tx,
                    RuntimeRelayLevel::Warn,
                    "parallel_protocol",
                    &format!(
                        "lane probe shape failed host_family={} cap={} method={} query_key={} url_variant={} {} {}",
                        task.host_family,
                        task.capability_name,
                        task.request.method,
                        query_key,
                        task.url_variant,
                        err,
                        classified_url
                    ),
                );
            }
        }
    }
}

pub(super) fn summarize_non_baseline_caps_by_host(
    entries: &BTreeMap<String, String>,
    max_per_host: usize,
) -> String {
    if entries.is_empty() {
        return String::from("none");
    }
    let baseline = OBJECT_INGRESS_BASELINE_CAP_NAMES
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut by_host = BTreeMap::<String, Vec<String>>::new();
    for (name, url) in entries {
        if baseline.contains(name.as_str()) {
            continue;
        }
        let classification = classify_capability_url(url);
        let host_family = classification
            .host
            .as_deref()
            .and_then(|host| host.split('.').next())
            .unwrap_or("unknown-host")
            .to_string();
        by_host.entry(host_family).or_default().push(name.clone());
    }
    if by_host.is_empty() {
        return String::from("none");
    }

    by_host
        .into_iter()
        .map(|(host_family, mut names)| {
            names.sort();
            names.dedup();
            let shown = names
                .iter()
                .take(max_per_host.max(1))
                .cloned()
                .collect::<Vec<_>>();
            let mut part = format!("{host_family}:count={}", names.len());
            if shown.is_empty() {
                part.push_str(" names=none");
                return part;
            }
            part.push_str(" names=");
            part.push_str(&shown.join(","));
            if names.len() > shown.len() {
                part.push_str(&format!(" (+{} more)", names.len() - shown.len()));
            }
            part
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) fn summarize_seed_capability_inventory(
    entries: &[SeedCapabilityInventoryEntry],
) -> String {
    if entries.is_empty() {
        return String::from("none");
    }

    let mut family_counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        let family = format_capability_url_family(entry.classification.family).to_string();
        *family_counts.entry(family).or_default() += 1;
    }

    let families = family_counts
        .into_iter()
        .map(|(family, count)| format!("{family}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    let entries_text = entries
        .iter()
        .map(|entry| {
            let host = entry.classification.host.as_deref().unwrap_or("?");
            let family = format_capability_url_family(entry.classification.family);
            match entry.classification.port {
                Some(port) => format!("{}={family}@{host}:{port}", entry.name),
                None => format!("{}={family}@{host}", entry.name),
            }
        })
        .collect::<Vec<_>>()
        .join(";");
    format!("families={families} entries={entries_text}")
}

#[derive(Debug, Clone, Default)]
pub(super) struct CapabilityReadinessEntry {
    available: bool,
    invoked: usize,
    ok: usize,
    err: usize,
    last: Option<String>,
}

pub(super) fn init_capability_readiness(
    capabilities: Option<&viewer_net::SeedCapabilityMap>,
) -> BTreeMap<String, CapabilityReadinessEntry> {
    let tracked = [
        "EventQueueGet",
        "InterestList",
        "UntrustedSimulatorMessage",
        "RegionObjects",
    ];
    let mut readiness = BTreeMap::new();
    for name in tracked {
        let available = capabilities
            .and_then(|caps| caps.entries.get(name))
            .is_some();
        readiness.insert(
            String::from(name),
            CapabilityReadinessEntry {
                available,
                ..Default::default()
            },
        );
    }
    readiness
}

pub(super) fn mark_capability_invocation_started(
    readiness: &mut BTreeMap<String, CapabilityReadinessEntry>,
    name: &str,
) {
    let entry = readiness.entry(String::from(name)).or_default();
    entry.invoked = entry.invoked.saturating_add(1);
    entry.last = Some(String::from("invoked"));
}

pub(super) fn mark_capability_invocation_result(
    readiness: &mut BTreeMap<String, CapabilityReadinessEntry>,
    name: &str,
    result: Result<&str, &str>,
) {
    let entry = readiness.entry(String::from(name)).or_default();
    match result {
        Ok(status) => {
            entry.ok = entry.ok.saturating_add(1);
            entry.last = Some(String::from(status));
        }
        Err(err) => {
            entry.err = entry.err.saturating_add(1);
            entry.last = Some(format!("err:{err}"));
        }
    }
}

pub(super) async fn run_pending_capability_readiness_probes(
    connection: &mut Connection,
    capability_readiness: &mut BTreeMap<String, CapabilityReadinessEntry>,
    protocol_events: &mut Vec<String>,
    pending_interest_list_probe_url: &mut Option<String>,
    pending_untrusted_simulator_message_probe_url: &mut Option<String>,
) {
    if let Some(url) = pending_interest_list_probe_url.take() {
        run_interest_list_probe_once(connection, capability_readiness, protocol_events, &url).await;
    }
    if let Some(url) = pending_untrusted_simulator_message_probe_url.take() {
        run_untrusted_simulator_message_probe_once(
            connection,
            capability_readiness,
            protocol_events,
            &url,
        )
        .await;
    }
}

async fn run_interest_list_probe_once(
    connection: &mut Connection,
    capability_readiness: &mut BTreeMap<String, CapabilityReadinessEntry>,
    protocol_events: &mut Vec<String>,
    url: &str,
) {
    mark_capability_invocation_started(capability_readiness, "InterestList");
    push_protocol_event(
        protocol_events,
        format!("InterestList:start {}", format_classified_url(url)),
    );
    match connection.fetch_interest_list_once(url).await {
        Ok(inspection) => {
            mark_capability_invocation_result(capability_readiness, "InterestList", Ok("ok"));
            push_protocol_event(
                protocol_events,
                format!(
                    "InterestList:ok keys={} scalar={} complex={}",
                    summarize_key_list(&inspection.top_level_keys),
                    summarize_scalar_map(&inspection.scalar_values),
                    summarize_scalar_map(&inspection.complex_value_types)
                ),
            );
        }
        Err(err) => {
            let err_text = err.to_string();
            mark_capability_invocation_result(capability_readiness, "InterestList", Err(&err_text));
            push_protocol_event(protocol_events, format!("InterestList:err {err_text}"));
        }
    }
}

async fn run_untrusted_simulator_message_probe_once(
    connection: &mut Connection,
    capability_readiness: &mut BTreeMap<String, CapabilityReadinessEntry>,
    protocol_events: &mut Vec<String>,
    url: &str,
) {
    mark_capability_invocation_started(capability_readiness, "UntrustedSimulatorMessage");
    push_protocol_event(
        protocol_events,
        format!(
            "UntrustedSimulatorMessage:start {}",
            format_classified_url(url)
        ),
    );
    match connection.fetch_untrusted_simulator_message_once(url).await {
        Ok(inspection) => {
            mark_capability_invocation_result(
                capability_readiness,
                "UntrustedSimulatorMessage",
                Ok("ok"),
            );
            push_protocol_event(
                protocol_events,
                format!(
                    "UntrustedSimulatorMessage:ok keys={} scalar={} complex={}",
                    summarize_key_list(&inspection.top_level_keys),
                    summarize_scalar_map(&inspection.scalar_values),
                    summarize_scalar_map(&inspection.complex_value_types)
                ),
            );
        }
        Err(err) => {
            let err_text = err.to_string();
            mark_capability_invocation_result(
                capability_readiness,
                "UntrustedSimulatorMessage",
                Err(&err_text),
            );
            push_protocol_event(
                protocol_events,
                format!("UntrustedSimulatorMessage:err {err_text}"),
            );
        }
    }
}

pub(super) fn summarize_capability_readiness(
    readiness: &BTreeMap<String, CapabilityReadinessEntry>,
) -> String {
    if readiness.is_empty() {
        return String::from("none");
    }
    readiness
        .iter()
        .map(|(name, entry)| {
            let available = if entry.available { "yes" } else { "no" };
            let last = entry.last.as_deref().unwrap_or("none");
            format!(
                "{name}(a={available},i={},ok={},err={},last={last})",
                entry.invoked, entry.ok, entry.err
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub(super) fn summarize_region_objects_inspection(inspection: &RegionObjectsInspection) -> String {
    let keys = if inspection.top_level_keys.is_empty() {
        String::from("none")
    } else {
        inspection
            .top_level_keys
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    };
    let arrays = if inspection.array_lengths.is_empty() {
        String::from("none")
    } else {
        inspection
            .array_lengths
            .iter()
            .take(4)
            .map(|(key, len)| format!("{key}:{len}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let first_item_keys = if inspection.first_array_item_keys.is_empty() {
        String::from("none")
    } else {
        inspection
            .first_array_item_keys
            .iter()
            .take(3)
            .map(|(key, item_keys)| {
                let joined = item_keys
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("|");
                format!("{key}={joined}")
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let complex = if inspection.complex_value_types.is_empty() {
        String::from("none")
    } else {
        inspection
            .complex_value_types
            .iter()
            .take(4)
            .map(|(key, value_type)| format!("{key}={value_type}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let scalars = if inspection.scalar_values.is_empty() {
        String::from("none")
    } else {
        inspection
            .scalar_values
            .iter()
            .take(4)
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let child_keys = if inspection.child_map_keys.is_empty() {
        String::from("none")
    } else {
        inspection
            .child_map_keys
            .iter()
            .take(3)
            .map(|(key, child_keys)| {
                let joined = child_keys
                    .iter()
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("|");
                format!("{key}={joined}")
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let child_scalars = if inspection.child_map_scalar_values.is_empty() {
        String::from("none")
    } else {
        inspection
            .child_map_scalar_values
            .iter()
            .take(3)
            .map(|(key, child_scalars)| {
                let joined = child_scalars
                    .iter()
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("|");
                format!("{key}={joined}")
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let child_profiles = if inspection.child_map_profiles.is_empty() {
        String::from("none")
    } else {
        inspection
            .child_map_profiles
            .iter()
            .take(3)
            .map(|(key, profile)| format!("{key}={profile}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let child_semantics = if inspection.child_map_semantic_values.is_empty() {
        String::from("none")
    } else {
        inspection
            .child_map_semantic_values
            .iter()
            .take(3)
            .map(|(key, semantic_values)| {
                let joined = semantic_values
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("|");
                format!("{key}={joined}")
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let child_typed = if inspection.child_map_pathfinding_summaries.is_empty() {
        String::from("none")
    } else {
        inspection
            .child_map_pathfinding_summaries
            .iter()
            .take(3)
            .map(|(key, summary)| {
                let mut parts = Vec::new();
                parts.push(format!("profile={}", summary.profile));
                if let Some(variant_hint) = &summary.variant_hint {
                    parts.push(format!("variant={variant_hint}"));
                }
                if let Some(linkset_use) = &summary.linkset_use {
                    parts.push(format!("linkset_use={linkset_use}"));
                }
                if let Some([a, b, c, d]) = summary.walkability_coefficients {
                    parts.push(format!("walkability={a}/{b}/{c}/{d}"));
                }
                if let Some(name) = &summary.name {
                    parts.push(format!("name={name}"));
                }
                if let Some(position) = &summary.position {
                    parts.push(format!("position={position}"));
                }
                if summary.position_key_present {
                    parts.push(String::from("position_key=present"));
                }
                if let Some(position_shape) = &summary.position_shape {
                    parts.push(format!("position_shape={position_shape}"));
                }
                if let Some(description_shape) = &summary.description_shape {
                    parts.push(format!("description_shape={description_shape}"));
                }
                if let Some(description_numeric_tuple) = &summary.description_numeric_tuple {
                    parts.push(format!(
                        "description_tuple={}",
                        description_numeric_tuple.join("|")
                    ));
                }
                if let Some(description) = &summary.description {
                    parts.push(format!("description={description}"));
                }
                if let Some(owner) = &summary.owner {
                    parts.push(format!("owner={owner}"));
                }
                if let Some(landimpact) = summary.landimpact {
                    parts.push(format!("landimpact={landimpact}"));
                }
                if let Some(navmesh_category) = summary.navmesh_category {
                    parts.push(format!("navmesh_category={navmesh_category}"));
                }
                if let Some(can_be_volume) = summary.can_be_volume {
                    parts.push(format!("can_be_volume={can_be_volume}"));
                }
                if let Some(phantom) = summary.phantom {
                    parts.push(format!("phantom={phantom}"));
                }
                format!(
                    "{key}={}",
                    parts.into_iter().take(10).collect::<Vec<_>>().join("|")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let tuple_analysis = if let Some(analysis) = &inspection.tuple_description_analysis {
        let slot_summary = if analysis.slot_distinct_values.is_empty() {
            String::from("none")
        } else {
            analysis
                .slot_distinct_values
                .iter()
                .enumerate()
                .map(|(idx, values)| {
                    if values.is_empty() {
                        format!("s{idx}=none")
                    } else if values.len() == 1 {
                        format!("s{idx}=const:{}", values.join("|"))
                    } else {
                        format!("s{idx}=var:{}", values.join("|"))
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        let samples = if analysis.sample_pairs.is_empty() {
            String::from("none")
        } else {
            analysis.sample_pairs.join(";")
        };
        let names = if analysis.distinct_names.is_empty() {
            String::from("none")
        } else {
            analysis.distinct_names.join("|")
        };
        format!(
            "samples={} slots={} names={} slot_values={} sample_pairs={}",
            analysis.sample_count, analysis.slot_count, names, slot_summary, samples
        )
    } else {
        String::from("none")
    };
    let typed_sample = if inspection.typed_object_samples.is_empty() {
        String::from("none")
    } else {
        inspection
            .typed_object_samples
            .iter()
            .take(3)
            .map(|sample| {
                let mut parts = Vec::new();
                parts.push(format!("profile={}", sample.profile));
                if let Some(name) = &sample.name {
                    parts.push(format!("name={name}"));
                }
                if let Some(linkset_use) = &sample.linkset_use {
                    parts.push(format!("linkset_use={linkset_use}"));
                }
                if let Some([a, b, c, d]) = sample.walkability_coefficients {
                    parts.push(format!("walkability={a}/{b}/{c}/{d}"));
                }
                if let Some(position) = &sample.position {
                    parts.push(format!("position={position}"));
                }
                if let Some(description_shape) = &sample.description_shape {
                    parts.push(format!("description_shape={description_shape}"));
                }
                if let Some(landimpact) = sample.landimpact {
                    parts.push(format!("landimpact={landimpact}"));
                }
                if let Some(owner) = &sample.owner {
                    parts.push(format!("owner={owner}"));
                }
                format!(
                    "{}={}",
                    sample.object_id,
                    parts.into_iter().take(8).collect::<Vec<_>>().join("|")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let mesh_candidates = if inspection.candidate_mesh_asset_ids.is_empty() {
        String::from("none")
    } else {
        inspection
            .candidate_mesh_asset_ids
            .iter()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "keys={keys} arrays={arrays} complex={complex} first_item_keys={first_item_keys} scalars={scalars} child_keys={child_keys} child_scalars={child_scalars} child_profiles={child_profiles} child_semantics={child_semantics} child_typed={child_typed} typed_sample={typed_sample} tuple_analysis={tuple_analysis} mesh_candidates={mesh_candidates}"
    )
}

pub(super) fn summarize_key_list(keys: &[String]) -> String {
    if keys.is_empty() {
        return String::from("none");
    }
    keys.iter()
        .take(8)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn summarize_scalar_map(values: &BTreeMap<String, String>) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .take(8)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(";")
}

#[derive(Debug)]
pub(super) struct SessionResidencyClassification {
    pub(super) state: &'static str,
    pub(super) evidence: Vec<String>,
}

pub(super) fn classify_session_residency(
    timeline: &viewer_net::FirstSimulatorStartupTimelineSummary,
    region: &viewer_net::RegionTransitionControlSummary,
    event_queue_has_observed_ok: bool,
    event_queue_consecutive_failures: u32,
    event_queue_cap_not_found_failures: u32,
    event_queue_simulator_target_messages: usize,
    event_queue_enable_simulator_messages: usize,
) -> SessionResidencyClassification {
    let mut evidence = Vec::new();
    evidence.push(format!(
        "amc={}",
        format_optional_index(timeline.first_agent_movement_complete_index)
    ));
    evidence.push(format!(
        "region_handshake={}",
        format_optional_index(timeline.first_region_handshake_index)
    ));
    evidence.push(format!(
        "region_handshake_reply={}",
        format_optional_index(timeline.first_region_handshake_reply_index)
    ));
    evidence.push(format!("eq_ok={event_queue_has_observed_ok}"));
    evidence.push(format!("eq_failures={event_queue_consecutive_failures}"));
    evidence.push(format!(
        "eq_cap_not_found={event_queue_cap_not_found_failures}"
    ));
    evidence.push(format!(
        "eq_sim_targets={event_queue_simulator_target_messages}"
    ));
    evidence.push(format!(
        "eq_enable_simulator={event_queue_enable_simulator_messages}"
    ));
    evidence.push(format!("region_crossed={}", region.crossed_region));
    evidence.push(format!(
        "region_confirm_enable={}",
        region.confirm_enable_simulator
    ));

    let state = if event_queue_consecutive_failures > 0 || event_queue_cap_not_found_failures > 0 {
        "degraded"
    } else if region.crossed_region > 0
        || region.confirm_enable_simulator > 0
        || event_queue_enable_simulator_messages > 0
    {
        "child_likely"
    } else if timeline.first_agent_movement_complete_index.is_some()
        && timeline.first_region_handshake_index.is_some()
        && event_queue_has_observed_ok
    {
        "root_likely"
    } else {
        "unknown"
    };

    SessionResidencyClassification { state, evidence }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_parallel_protocol_summary(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    connection: &Connection,
    label: &str,
    capability_inventory: &str,
    capability_readiness: &str,
    protocol_events: &[String],
    event_queue_url: Option<&str>,
    event_ack: u64,
    event_queue_consecutive_failures: u32,
    event_queue_cap_not_found_failures: u32,
    event_queue_has_observed_ok: bool,
    event_queue_simulator_target_messages: usize,
    event_queue_enable_simulator_messages: usize,
) {
    let region = connection.summarize_region_transition_control();
    let timeline = connection.summarize_first_simulator_startup_timeline();
    let event_queue = event_queue_url
        .map(format_classified_url)
        .unwrap_or_else(|| String::from("none"));
    let residency = classify_session_residency(
        &timeline,
        &region,
        event_queue_has_observed_ok,
        event_queue_consecutive_failures,
        event_queue_cap_not_found_failures,
        event_queue_simulator_target_messages,
        event_queue_enable_simulator_messages,
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "parallel_protocol",
        &format!("{label} caps: {capability_inventory}"),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "parallel_protocol",
        &format!("{label} readiness: {capability_readiness}"),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "parallel_protocol",
        &format!(
            "{label} flow: event_queue_url={event_queue} event_ack={} eq_failures={} eq_cap_not_found={} region_ctrl={} crossed={} confirm={} events={}",
            event_ack,
            event_queue_consecutive_failures,
            event_queue_cap_not_found_failures,
            region.observations,
            region.crossed_region,
            region.confirm_enable_simulator,
            format_transcript_side(protocol_events),
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "parallel_protocol",
        &format!(
            "{label} residency_state={} evidence={}",
            residency.state,
            residency.evidence.join(";")
        ),
    );
}

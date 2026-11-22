//! Device identity that survives reconnects (D-16).
//!
//! Bindings (hotkeys, toggle pairs, profiles) store a [`DeviceRef`], never an index or a bare name.
//! [`resolve`] maps it to a live endpoint using, in order:
//!
//! 1. the saved endpoint ID, if that endpoint is **Active**;
//! 2. the saved `PKEY_AudioEndpoint_StableId`, if an Active endpoint has it;
//! 3. (`SameModel` only) a hardware match: same hardware key and/or container **and** the same
//!    *logical* endpoint (form factor + endpoint name). A unique match is re-linked; anything
//!    uncertain is never guessed.
//!
//! Only Active endpoints are ever considered: Windows keeps NotPresent "ghosts" with identical names.
//! Names are only a weak signal, because Windows renames re-installed devices
//! ("Arctis Nova 7" → "2- Arctis Nova 7", seen on the owner's PC).

use serde::{Deserialize, Serialize};

use crate::device::{DeviceState, EndpointInfo, Flow};

/// How far a binding may follow its device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// Only this exact endpoint (endpoint ID or stable ID). UI: "This exact device/port".
    Exact,
    /// The same model's equivalent endpoint, even after Windows re-created it. UI: "Any port".
    #[default]
    SameModel,
}

/// A saved reference to an endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRef {
    pub endpoint_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_id: Option<String>,
    pub flow: Flow,
    /// Shown to the user when the device is missing, e.g. "Headphones (Arctis Nova 7)".
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_name: Option<String>,
    /// Normalised (without Windows' `N- ` re-install prefix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_factor: Option<u32>,
    #[serde(default)]
    pub match_mode: MatchMode,
}

impl DeviceRef {
    /// Captures everything needed to find this endpoint again.
    #[must_use]
    pub fn from_endpoint(e: &EndpointInfo, match_mode: MatchMode) -> Self {
        let interface_name = e.interface_name.as_deref().map(normalize_name);
        let display_name = match (&e.endpoint_name, &interface_name) {
            (Some(ep), Some(ifc)) => format!("{ep} ({ifc})"),
            _ => e.friendly_name.as_deref().map(normalize_name).unwrap_or_else(|| e.id.clone()),
        };
        Self {
            endpoint_id: e.id.clone(),
            stable_id: e.stable_id.clone(),
            flow: e.flow,
            display_name,
            endpoint_name: e.endpoint_name.clone(),
            interface_name,
            hardware_key: e.hardware_key.clone(),
            container_id: e.container_id.clone(),
            form_factor: e.form_factor,
            match_mode,
        }
    }

    /// The same binding pointed at a re-created endpoint. Persist it only after the switch is
    /// confirmed (D-16 amendment).
    #[must_use]
    pub fn relinked_to(&self, e: &EndpointInfo) -> Self {
        Self {
            match_mode: self.match_mode,
            display_name: self.display_name.clone(),
            ..Self::from_endpoint(e, self.match_mode)
        }
    }
}

/// Why a [`DeviceRef`] resolved the way it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution<'a> {
    /// The saved endpoint ID is present and Active.
    Exact(&'a EndpointInfo),
    /// Found by `PKEY_AudioEndpoint_StableId`; the endpoint ID changed.
    StableId(&'a EndpointInfo),
    /// Found by hardware + logical-endpoint match. Re-link after the switch is confirmed.
    Relinked(&'a EndpointInfo),
    /// Same hardware, but we can't be sure it's the same logical output (e.g. missing form factor).
    /// Ask the user once.
    NeedsConfirmation(&'a EndpointInfo),
    /// Two or more equally good candidates (e.g. two identical headsets). Show a picker.
    Ambiguous,
    /// Nothing suitable is connected. Tell the user; never fail silently.
    NotConnected,
}

impl<'a> Resolution<'a> {
    /// The endpoint to switch to without asking, if any.
    #[must_use]
    pub fn target(&self) -> Option<&'a EndpointInfo> {
        match *self {
            Self::Exact(e) | Self::StableId(e) | Self::Relinked(e) => Some(e),
            _ => None,
        }
    }
}

/// Strips Windows' re-install prefix: `"2- Arctis Nova 7"` → `"Arctis Nova 7"`,
/// `"Headphones (2- Arctis Nova 7)"` → `"Headphones (Arctis Nova 7)"`.
#[must_use]
pub fn normalize_name(name: &str) -> String {
    fn strip(s: &str) -> &str {
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && s[digits..].starts_with("- ") { &s[digits + 2..] } else { s }
    }
    let name = name.trim();
    match name.split_once(" (") {
        Some((head, tail)) => format!("{} ({}", strip(head), strip(tail)),
        None => strip(name).to_string(),
    }
}

/// Built-in and virtual devices all share Windows' placeholder container
/// `{00000000-0000-0000-FFFF-FFFFFFFFFFFF}` (seen on the owner's Realtek and Steam endpoints).
/// It identifies nothing, so treat it as "no container".
fn real_container(c: Option<&str>) -> Option<&str> {
    c.filter(|c| !c.trim_matches(['{', '}']).eq_ignore_ascii_case("00000000-0000-0000-FFFF-FFFFFFFFFFFF"))
}

fn eq_ci(a: Option<&str>, b: Option<&str>) -> Option<bool> {
    Some(a?.trim().eq_ignore_ascii_case(b?.trim()))
}

/// Resolves a saved reference against the current endpoint list (any states; only Active are used).
#[must_use]
pub fn resolve<'a>(r: &DeviceRef, endpoints: &'a [EndpointInfo]) -> Resolution<'a> {
    let live: Vec<&EndpointInfo> =
        endpoints.iter().filter(|e| e.state == DeviceState::Active && e.flow == r.flow).collect();

    if let Some(e) = live.iter().find(|e| e.id.eq_ignore_ascii_case(&r.endpoint_id)) {
        return Resolution::Exact(e);
    }
    if let Some(sid) = &r.stable_id
        && let Some(e) = live.iter().find(|e| e.stable_id.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(sid)))
    {
        return Resolution::StableId(e);
    }
    if r.match_mode == MatchMode::Exact {
        return Resolution::NotConnected;
    }

    let (confirmed, uncertain) = classify_candidates(r, &live);
    match (confirmed.as_slice(), uncertain.as_slice()) {
        // Only a lone, logically confirmed match re-links automatically. If any uncertain candidate
        // also exists, never guess (review ENG-11).
        ([one], []) => Resolution::Relinked(one),
        ([], [one]) => Resolution::NeedsConfirmation(one),
        ([], []) => Resolution::NotConnected,
        _ => Resolution::Ambiguous,
    }
}

/// Endpoints a picker should offer when [`resolve`] returns `Ambiguous` or `NeedsConfirmation`:
/// same hardware anchor, not a different logical endpoint, Active, same flow. This is the single source of
/// truth for candidate rules (review ENG-06).
#[must_use]
pub fn candidates<'a>(r: &DeviceRef, endpoints: &'a [EndpointInfo]) -> Vec<&'a EndpointInfo> {
    let live: Vec<&EndpointInfo> =
        endpoints.iter().filter(|e| e.state == DeviceState::Active && e.flow == r.flow).collect();
    let (mut confirmed, uncertain) = classify_candidates(r, &live);
    confirmed.extend(uncertain);
    confirmed
}

/// Splits hardware-anchored live endpoints into logically confirmed and uncertain matches
/// (logically different ones are dropped).
fn classify_candidates<'a>(r: &DeviceRef, live: &[&'a EndpointInfo]) -> (Vec<&'a EndpointInfo>, Vec<&'a EndpointInfo>) {
    // Hardware anchor: without one we never re-link by name alone.
    let anchored = |e: &&&EndpointInfo| {
        let hw = eq_ci(r.hardware_key.as_deref(), e.hardware_key.as_deref());
        let ct = eq_ci(real_container(r.container_id.as_deref()), real_container(e.container_id.as_deref()));
        // A known container that differs is a different physical unit, even for the same model.
        hw == Some(true) && ct != Some(false) || ct == Some(true) && hw != Some(false)
    };
    // Logical endpoint: a headset's "Headphones" output must not re-link onto its "Headset" (hands-free) output.
    let logical = |e: &EndpointInfo| -> Option<bool> {
        let ff = match (r.form_factor, e.form_factor) {
            (Some(a), Some(b)) => Some(a == b),
            _ => None,
        };
        let name = eq_ci(r.endpoint_name.as_deref(), e.endpoint_name.as_deref());
        match (ff, name) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None, // one signal missing: uncertain
        }
    };
    let mut confirmed = Vec::new();
    let mut uncertain = Vec::new();
    for e in live.iter().filter(anchored).copied() {
        match logical(e) {
            Some(true) => confirmed.push(e),
            None => uncertain.push(e),
            Some(false) => {}
        }
    }
    (confirmed, uncertain)
}

/// What a toggle-pair hotkey (e.g. Speakers ↔ Arctis) should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleAction<'a> {
    /// Switch to this side of the pair (0 = primary, 1 = secondary).
    SwitchTo { side: usize, resolution: Resolution<'a> },
    /// The side we need isn't usable; keep the current device and say why.
    Unavailable { side: usize, resolution: Resolution<'a> },
}

/// Decides a toggle press. If the current default is one side, go to the other. If a third device
/// took over (e.g. a monitor after display wake), go to the primary side, or the secondary if the
/// primary isn't connected (Gemini's D-16 amendment).
#[must_use]
pub fn toggle<'a>(
    pair: [&DeviceRef; 2],
    current_default_id: Option<&str>,
    endpoints: &'a [EndpointInfo],
) -> ToggleAction<'a> {
    let res = [resolve(pair[0], endpoints), resolve(pair[1], endpoints)];
    let is_current =
        |r: &Resolution<'_>| r.target().zip(current_default_id).is_some_and(|(e, cur)| e.id.eq_ignore_ascii_case(cur));
    let side = if is_current(&res[0]) {
        1
    } else if is_current(&res[1]) || (res[0].target().is_none() && res[1].target().is_some()) {
        if is_current(&res[1]) { 0 } else { 1 }
    } else {
        0
    };
    match res[side] {
        r if r.target().is_some() => ToggleAction::SwitchTo { side, resolution: r },
        r => ToggleAction::Unavailable { side, resolution: r },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::hardware_key_from_devnode;

    /// Builds an endpoint like the ones on the owner's PC (2026-11-22 scan).
    #[allow(clippy::too_many_arguments)]
    fn ep(
        id: &str,
        state: DeviceState,
        flow: Flow,
        endpoint: &str,
        iface: &str,
        devnode: &str,
        container: &str,
        ff: u32,
    ) -> EndpointInfo {
        EndpointInfo {
            id: id.into(),
            flow,
            state,
            friendly_name: Some(format!("{endpoint} ({iface})")),
            endpoint_name: Some(endpoint.into()),
            interface_name: Some(iface.into()),
            hardware_key: hardware_key_from_devnode(devnode),
            devnode: Some(devnode.into()),
            container_id: Some(container.into()),
            form_factor: Some(ff),
            stable_id: Some(format!("stable-{id}")),
            default_for: vec![],
        }
    }

    const ARCTIS_DEV: &str = r"{1}.USB\VID_1038&PID_22A1&MI_00\7&2B084728&0&0000";
    const ARCTIS_DEV_OTHER_PORT: &str = r"{1}.USB\VID_1038&PID_22A1&MI_00\7&99999999&0&0000";
    const REALTEK_DEV: &str = r"{1}.HDAUDIO\FUNC_01&VEN_10EC&DEV_0B00&SUBSYS_18491204&REV_1000\5&7AA6472&0&0001";
    const ARCTIS_CT: &str = "{8EC256AB-0000-0000-0000-000000000001}";
    const NULL_CT: &str = "{00000000-0000-0000-FFFF-FFFFFFFFFFFF}";

    fn arctis(id: &str, state: DeviceState) -> EndpointInfo {
        ep(id, state, Flow::Render, "Headphones", "Arctis Nova 7", ARCTIS_DEV, ARCTIS_CT, 3)
    }
    fn realtek(id: &str, state: DeviceState) -> EndpointInfo {
        ep(id, state, Flow::Render, "Speakers", "Realtek(R) Audio", REALTEK_DEV, NULL_CT, 1)
    }
    fn arctis_mic() -> EndpointInfo {
        ep("mic", DeviceState::Active, Flow::Capture, "Microphone", "Arctis Nova 7", ARCTIS_DEV, ARCTIS_CT, 4)
    }

    #[test]
    fn normalizes_reinstall_prefix() {
        assert_eq!(normalize_name("2- Arctis Nova 7"), "Arctis Nova 7");
        assert_eq!(normalize_name("Headphones (2- Arctis Nova 7)"), "Headphones (Arctis Nova 7)");
        assert_eq!(normalize_name("12- USB DAC"), "USB DAC");
        assert_eq!(normalize_name("Speakers (Realtek(R) Audio)"), "Speakers (Realtek(R) Audio)");
        assert_eq!(normalize_name("1 - Digital Display"), "1 - Digital Display");
    }

    #[test]
    fn exact_id_wins() {
        let list = vec![arctis("a", DeviceState::Active), realtek("r", DeviceState::Active)];
        let r = DeviceRef::from_endpoint(&list[0], MatchMode::SameModel);
        assert_eq!(resolve(&r, &list), Resolution::Exact(&list[0]));
    }

    #[test]
    fn ghost_with_same_id_is_never_used() {
        // The legacy app's bug: it resolved to NotPresent duplicates.
        let list = vec![arctis("a", DeviceState::NotPresent)];
        let r = DeviceRef::from_endpoint(&arctis("a", DeviceState::Active), MatchMode::SameModel);
        assert_eq!(resolve(&r, &list), Resolution::NotConnected);
    }

    #[test]
    fn stable_id_finds_new_endpoint_id() {
        let old = arctis("old", DeviceState::Active);
        let mut new = arctis("new", DeviceState::Active);
        new.stable_id = old.stable_id.clone();
        let list = vec![arctis("old", DeviceState::NotPresent), new];
        let r = DeviceRef::from_endpoint(&old, MatchMode::Exact);
        assert_eq!(resolve(&r, &list), Resolution::StableId(&list[1]));
    }

    #[test]
    fn other_port_relinks_same_model() {
        // Different USB port: new endpoint ID, new stable ID, new instance; same hardware key + container.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let mut moved = ep(
            "new",
            DeviceState::Active,
            Flow::Render,
            "Headphones",
            "2- Arctis Nova 7",
            ARCTIS_DEV_OTHER_PORT,
            ARCTIS_CT,
            3,
        );
        moved.stable_id = Some("stable-new".into());
        let list = vec![arctis("old", DeviceState::NotPresent), moved, realtek("r", DeviceState::Active), arctis_mic()];
        assert_eq!(resolve(&saved, &list), Resolution::Relinked(&list[1]));
        let relinked = saved.relinked_to(&list[1]);
        assert_eq!(relinked.endpoint_id, "new");
        assert_eq!(relinked.interface_name.as_deref(), Some("Arctis Nova 7"));
        assert_eq!(relinked.display_name, saved.display_name);
    }

    #[test]
    fn exact_mode_does_not_follow_other_port() {
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::Exact);
        let list = vec![ep(
            "new",
            DeviceState::Active,
            Flow::Render,
            "Headphones",
            "2- Arctis Nova 7",
            ARCTIS_DEV_OTHER_PORT,
            ARCTIS_CT,
            3,
        )];
        assert_eq!(resolve(&saved, &list), Resolution::NotConnected);
    }

    #[test]
    fn never_crosses_flow_or_logical_endpoint() {
        // Headphones must not re-link onto the same headset's hands-free output or its microphone.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let hands_free =
            ep("hf", DeviceState::Active, Flow::Render, "Headset", "Arctis Nova 7", ARCTIS_DEV, ARCTIS_CT, 5);
        let list = vec![hands_free, arctis_mic()];
        assert_eq!(resolve(&saved, &list), Resolution::NotConnected);
    }

    #[test]
    fn missing_form_factor_asks_instead_of_guessing() {
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let mut e = arctis("new", DeviceState::Active);
        e.stable_id = None;
        e.form_factor = None;
        let list = vec![e];
        assert_eq!(resolve(&saved, &list), Resolution::NeedsConfirmation(&list[0]));
    }

    #[test]
    fn two_identical_headsets_are_ambiguous_without_container() {
        let mut saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        saved.container_id = None;
        let mut a = arctis("a", DeviceState::Active);
        let mut b = ep(
            "b",
            DeviceState::Active,
            Flow::Render,
            "Headphones",
            "Arctis Nova 7",
            ARCTIS_DEV_OTHER_PORT,
            "{OTHER}",
            3,
        );
        a.stable_id = None;
        b.stable_id = None;
        let list = vec![a, b];
        assert_eq!(resolve(&saved, &list), Resolution::Ambiguous);
    }

    #[test]
    fn different_unit_of_same_model_is_not_relinked() {
        // Same hardware key, but a known, different container = another physical headset.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let list = vec![ep(
            "b",
            DeviceState::Active,
            Flow::Render,
            "Headphones",
            "Arctis Nova 7",
            ARCTIS_DEV_OTHER_PORT,
            "{OTHER}",
            3,
        )];
        assert_eq!(resolve(&saved, &list), Resolution::NotConnected);
    }

    #[test]
    fn confirmed_plus_uncertain_is_ambiguous_not_a_guess() {
        // Review ENG-11: one confirmed match must not auto-relink while another uncertain candidate exists.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let mut good = arctis("a", DeviceState::Active);
        good.stable_id = None;
        let mut unsure = arctis("b", DeviceState::Active);
        unsure.stable_id = None;
        unsure.form_factor = None;
        let list = vec![good, unsure];
        assert_eq!(resolve(&saved, &list), Resolution::Ambiguous);
        let offered: Vec<&str> = candidates(&saved, &list).iter().map(|e| e.id.as_str()).collect();
        assert_eq!(offered, ["a", "b"]);
    }

    #[test]
    fn candidates_exclude_placeholder_container_and_other_logical_endpoints() {
        // Review ENG-06: the picker must follow the same rules as the resolver.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let hands_free =
            ep("hf", DeviceState::Active, Flow::Render, "Headset", "Arctis Nova 7", ARCTIS_DEV, ARCTIS_CT, 5);
        let list = vec![hands_free, realtek("r", DeviceState::Active), arctis_mic()];
        assert!(candidates(&saved, &list).is_empty());
    }

    #[test]
    fn placeholder_container_is_not_an_anchor() {
        // Realtek speakers and a Steam virtual device share the placeholder container; the speakers'
        // binding must never re-link onto the virtual device.
        let saved = DeviceRef::from_endpoint(&realtek("r", DeviceState::Active), MatchMode::SameModel);
        let mut steam = ep(
            "steam",
            DeviceState::Active,
            Flow::Render,
            "Speakers",
            "Steam Streaming Speakers",
            r"{1}.ROOT\STEAMSTREAMINGSPEAKERS\0000",
            NULL_CT,
            1,
        );
        steam.hardware_key = None;
        steam.stable_id = None;
        assert_eq!(resolve(&saved, &[steam]), Resolution::NotConnected);
    }

    #[test]
    fn name_alone_never_relinks() {
        // Virtual/unknown device with a matching name but no hardware anchor.
        let saved = DeviceRef::from_endpoint(&arctis("old", DeviceState::Active), MatchMode::SameModel);
        let mut fake = arctis("x", DeviceState::Active);
        fake.hardware_key = Some(r"ROOT\MEDIA".into());
        fake.container_id = Some("{VIRTUAL}".into());
        fake.stable_id = None;
        assert_eq!(resolve(&saved, &[fake]), Resolution::NotConnected);
    }

    #[test]
    fn toggle_pair_flips_and_handles_third_device() {
        let a = arctis("a", DeviceState::Active);
        let s = realtek("r", DeviceState::Active);
        let list = vec![
            a.clone(),
            s.clone(),
            ep(
                "lg",
                DeviceState::Active,
                Flow::Render,
                "LG FULL HD",
                "NVIDIA High Definition Audio",
                r"{1}.HDAUDIO\FUNC_01&VEN_10DE&DEV_009F\x",
                "{LG}",
                9,
            ),
        ];
        let pa = DeviceRef::from_endpoint(&a, MatchMode::SameModel); // primary: headset
        let ps = DeviceRef::from_endpoint(&s, MatchMode::SameModel);
        let side = |act: ToggleAction<'_>| match act {
            ToggleAction::SwitchTo { side, .. } => Some(side),
            ToggleAction::Unavailable { .. } => None,
        };
        assert_eq!(side(toggle([&pa, &ps], Some("a"), &list)), Some(1));
        assert_eq!(side(toggle([&pa, &ps], Some("r"), &list)), Some(0));
        assert_eq!(side(toggle([&pa, &ps], Some("lg"), &list)), Some(0), "third device → primary");
        // Headset off: from the monitor, fall back to speakers; from speakers, report the headset missing.
        let no_headset = vec![s.clone(), list[2].clone()];
        assert_eq!(side(toggle([&pa, &ps], Some("lg"), &no_headset)), Some(1));
        assert!(matches!(
            toggle([&pa, &ps], Some("r"), &no_headset),
            ToggleAction::Unavailable { side: 0, resolution: Resolution::NotConnected }
        ));
    }
}

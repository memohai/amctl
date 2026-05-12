use std::collections::BTreeSet;

const ANCHOR_CLASSES: &[&str] = &[
    "RecyclerView",
    "ListView",
    "ViewPager",
    "TabLayout",
    "Toolbar",
    "EditText",
    "WebView",
    "FrameLayout",
    "DialogTitle",
];

/// Rows from screen or refs used for fingerprint computation.
#[derive(Debug, Clone)]
pub struct FingerprintRow<'a> {
    pub class_name: Option<&'a str>,
    pub res_id: Option<&'a str>,
}

/// Build a stable, human-readable page fingerprint.
///
/// Format: `act=<activity>|mode=<mode>|wv=<0|1>|rid=<anchors>|cls=<classes>`
pub fn build_page_fingerprint(
    activity: &str,
    mode: &str,
    has_webview: bool,
    rows: &[FingerprintRow<'_>],
) -> String {
    let mut rid_anchors: BTreeSet<String> = BTreeSet::new();
    let mut cls_anchors: BTreeSet<String> = BTreeSet::new();

    for row in rows {
        if let Some(res_id) = row.res_id {
            let short = shorten_res_id(res_id);
            if !short.is_empty() {
                rid_anchors.insert(short);
            }
        }
        if let Some(class_name) = row.class_name {
            let short_class = shorten_class_name(class_name);
            if is_anchor_class(&short_class) {
                cls_anchors.insert(short_class);
            }
        }
    }

    let rid_anchors: Vec<_> = rid_anchors.into_iter().take(8).collect();
    let cls_anchors: Vec<_> = cls_anchors.into_iter().take(6).collect();

    let mut parts = Vec::with_capacity(5);
    parts.push(format!("act={activity}"));
    if !mode.is_empty() {
        parts.push(format!("mode={mode}"));
    }
    parts.push(format!("wv={}", if has_webview { 1 } else { 0 }));
    if !rid_anchors.is_empty() {
        parts.push(format!("rid={}", rid_anchors.join(",")));
    }
    if !cls_anchors.is_empty() {
        parts.push(format!("cls={}", cls_anchors.join(",")));
    }
    parts.join("|")
}

fn shorten_res_id(res_id: &str) -> String {
    let id = res_id
        .rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(res_id);
    if id.is_empty()
        || id == "content"
        || id == "statusBarBackground"
        || id == "navigationBarBackground"
        || id == "action_bar_root"
    {
        return String::new();
    }
    id.to_string()
}

fn shorten_class_name(class_name: &str) -> String {
    class_name
        .rsplit_once('.')
        .map(|(_, name)| name)
        .unwrap_or(class_name)
        .to_string()
}

fn is_anchor_class(short_class: &str) -> bool {
    ANCHOR_CLASSES
        .iter()
        .any(|class_name| short_class.contains(class_name))
}

//! Steward — self-husbandry under an operator-named fence.
//!
//! The landscape diagnoses; the steward *husbands*. It surveys a workspace
//! (typically this repository itself), names the open proposals whose facet
//! class the operator has explicitly **fenced**, and — only under `--apply`
//! — descends each chore as its own evidence-bound wish through the same
//! armament as wish mode and landscape adoption. The governance is the
//! house's: the machine proposes (survey and plan are read-only artifacts),
//! only the operator disposes (nothing is fenced by default; husbandry
//! without a fence is refused, and widening the fence is itself an explicit
//! operator act). The outcome is a content-addressed report fit for an
//! unattended nightly run — it names the workspace by digest, never by
//! host path.

use kosmo_core::{Digest, WishFacetKind};
use kosmo_pipeline::{LandscapeStanding, WishLandscape, WishProposal};

/// Every facet class the fence vocabulary can name, paired with its
/// operator-facing word. The fence speaks the wish vocabulary and nothing
/// else — there is no "all" shorthand, because an unbounded fence is not
/// a fence.
const FENCE_VOCABULARY: &[(&str, WishFacetKind)] = &[
    ("crate", WishFacetKind::Crate),
    ("module", WishFacetKind::Module),
    ("symbol", WishFacetKind::Symbol),
    ("capability", WishFacetKind::Capability),
    ("resolution", WishFacetKind::Resolution),
    ("dependency", WishFacetKind::Dependency),
    ("signature", WishFacetKind::Signature),
    ("contract", WishFacetKind::Contract),
    ("test", WishFacetKind::Test),
    ("doc", WishFacetKind::Doc),
    ("behavior", WishFacetKind::Behavior),
    ("composition", WishFacetKind::Composition),
    ("run", WishFacetKind::Run),
    ("service", WishFacetKind::Service),
];

fn kind_from_name(name: &str) -> Option<WishFacetKind> {
    FENCE_VOCABULARY
        .iter()
        .find(|(word, _)| *word == name)
        .map(|(_, kind)| kind.clone())
}

fn name_of_kind(kind: &WishFacetKind) -> &'static str {
    FENCE_VOCABULARY
        .iter()
        .find(|(_, k)| k == kind)
        .map(|(word, _)| *word)
        .unwrap_or("?")
}

/// The operator's fence: the facet classes the steward may husband. Parsed
/// from an explicit comma-separated spec; an empty or unknown spec is an
/// error, never a default — the fence exists only because the operator
/// spoke it.
#[derive(Clone, Debug)]
pub struct Fence {
    kinds: Vec<WishFacetKind>,
}

impl Fence {
    pub fn parse(spec: &str) -> Result<Fence, String> {
        let mut kinds = Vec::new();
        for word in spec.split(',') {
            let word = word.trim().to_lowercase();
            if word.is_empty() {
                continue;
            }
            let kind = kind_from_name(&word).ok_or_else(|| {
                format!(
                    "--fence: '{}' is not a facet class (the vocabulary: {})",
                    word,
                    FENCE_VOCABULARY
                        .iter()
                        .map(|(w, _)| *w)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        if kinds.is_empty() {
            return Err("--fence names no facet class (e.g. --fence doc,test)".into());
        }
        Ok(Fence { kinds })
    }

    pub fn contains(&self, kind: &WishFacetKind) -> bool {
        self.kinds.contains(kind)
    }

    /// The fence as the operator spoke it, normalized and deduplicated.
    pub fn names(&self) -> Vec<String> {
        self.kinds
            .iter()
            .map(|k| name_of_kind(k).to_string())
            .collect()
    }
}

/// The open proposals inside the fence, in landscape (severity) order,
/// optionally capped — the steward's chore list. Met proposals need no
/// husbandry; beyond-observation and unmeasured ones cannot be *measured*
/// to completion, so descending them would be motion without truth.
pub fn fenced_open<'a>(
    landscape: &'a WishLandscape,
    standing: &[LandscapeStanding],
    fence: &Fence,
    cap: Option<usize>,
) -> Vec<&'a WishProposal> {
    let chores = landscape
        .proposals
        .iter()
        .zip(standing)
        .filter(|(p, s)| **s == LandscapeStanding::Open && fence.contains(&p.facet.kind))
        .map(|(p, _)| p);
    match cap {
        Some(n) => chores.take(n).collect(),
        None => chores.collect(),
    }
}

/// One husbanded chore's outcome — a single-facet descent, observed.
#[derive(Debug, serde::Serialize)]
pub struct ChoreOutcome {
    pub kind: String,
    pub key: String,
    pub wish_id: String,
    pub realized: bool,
    pub iterations: usize,
}

/// The steward's report: survey counts, the fenced plan, and (under
/// `--apply`) the per-chore outcomes. Durable and host-path-free — the
/// workspace appears as its identity digest only.
#[derive(Debug, serde::Serialize)]
pub struct StewardReport {
    pub workspace: String,
    pub fence: Vec<String>,
    pub applied: bool,
    pub proposals: usize,
    pub met: usize,
    pub open: usize,
    pub beyond_observation: usize,
    pub beyond_vocabulary: usize,
    /// The fenced open chores (post-cap), as `"Kind key"` labels.
    pub planned: Vec<String>,
    pub chores: Vec<ChoreOutcome>,
}

impl StewardReport {
    /// The survey half of the report; chores are appended by the husbandry
    /// loop (and stay empty in a read-only run).
    pub fn survey(
        workspace_tag: Digest,
        fence: Option<&Fence>,
        landscape: &WishLandscape,
        standing: &[LandscapeStanding],
        chores: &[&WishProposal],
        applied: bool,
    ) -> Self {
        let count = |want: LandscapeStanding| standing.iter().filter(|s| **s == want).count();
        StewardReport {
            workspace: workspace_tag.to_hex(),
            fence: fence.map(Fence::names).unwrap_or_default(),
            applied,
            proposals: landscape.proposals.len(),
            met: count(LandscapeStanding::Met),
            open: count(LandscapeStanding::Open),
            beyond_observation: count(LandscapeStanding::BeyondObservation),
            beyond_vocabulary: landscape.unmapped.len(),
            planned: chores
                .iter()
                .map(|p| format!("{:?} {}", p.facet.kind, p.facet.key))
                .collect(),
            chores: Vec::new(),
        }
    }

    pub fn husbanded(&self) -> usize {
        self.chores.iter().filter(|c| c.realized).count()
    }

    /// Every descended chore realized (a read-only survey is trivially
    /// faithful — it claimed nothing).
    pub fn is_faithful(&self) -> bool {
        self.chores.iter().all(|c| c.realized)
    }

    /// The report as pretty JSON, prefixed by its own content address.
    pub fn to_json(&self) -> String {
        let body = serde_json::to_value(self).unwrap_or_default();
        let report_id = Digest::of(&body);
        serde_json::to_string_pretty(&serde_json::json!({
            "report_id": report_id.to_hex(),
            "report": body,
        }))
        .unwrap_or_default()
    }

    pub fn render(&self, color: bool) -> String {
        let (bold, green, red, yellow, dim, reset) = if color {
            (
                "\x1b[1m", "\x1b[32m", "\x1b[31m", "\x1b[33m", "\x1b[2m", "\x1b[0m",
            )
        } else {
            ("", "", "", "", "", "")
        };
        let mut out = format!("{bold}Kosmocrates steward — self-husbandry{reset}\n");
        out.push_str(&format!(
            "  workspace {dim}{}…{reset} · {} proposal(s) · {green}{} met{reset} · \
             {yellow}{} open{reset} · {} beyond observation · {} beyond vocabulary\n",
            &self.workspace[..12.min(self.workspace.len())],
            self.proposals,
            self.met,
            self.open,
            self.beyond_observation,
            self.beyond_vocabulary
        ));
        if self.fence.is_empty() {
            out.push_str(&format!(
                "  {dim}no fence named — survey only (name one with --fence doc,test){reset}\n"
            ));
            return out;
        }
        out.push_str(&format!(
            "  fence {bold}{}{reset} — {} open chore(s) inside\n",
            self.fence.join(", "),
            self.planned.len()
        ));
        // The rendered plan stays operator-sized; the JSON report carries
        // the full list.
        const PLAN_LINES: usize = 25;
        for label in self.planned.iter().take(PLAN_LINES) {
            out.push_str(&format!("    {label}\n"));
        }
        if self.planned.len() > PLAN_LINES {
            out.push_str(&format!(
                "    {dim}… and {} more (the report carries the full plan){reset}\n",
                self.planned.len() - PLAN_LINES
            ));
        }
        if !self.applied {
            out.push_str(&format!(
                "  {dim}(survey only — add --apply to husband the fenced chores){reset}\n"
            ));
            return out;
        }
        for c in &self.chores {
            let line = if c.realized {
                format!(
                    "  {green}\u{2713}{reset} {} {} — realized ({} iteration(s))",
                    c.kind, c.key, c.iterations
                )
            } else {
                format!(
                    "  {red}\u{2717}{reset} {} {} — NOT realized ({} iteration(s))",
                    c.kind, c.key, c.iterations
                )
            };
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str(&format!(
            "  husbanded: {}/{} fenced chore(s)\n",
            self.husbanded(),
            self.chores.len()
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{WishFacet, Q16};

    fn proposal(facet: WishFacet, severity: Q16) -> WishProposal {
        WishProposal {
            proposal_id: Digest::of_bytes(facet.key.as_bytes()),
            facet,
            void_id: Digest::of_bytes(b"void"),
            severity,
            subject: "demo".into(),
            rationale: "test fixture".into(),
        }
    }

    #[test]
    fn fence_parses_the_operator_vocabulary() {
        let fence = Fence::parse(" Doc , TEST ,doc").expect("valid fence");
        assert!(fence.contains(&WishFacetKind::Doc));
        assert!(fence.contains(&WishFacetKind::Test));
        assert!(!fence.contains(&WishFacetKind::Capability));
        // Duplicates collapse; order is the operator's.
        assert_eq!(fence.names(), vec!["doc", "test"]);
    }

    #[test]
    fn fence_refuses_unknown_and_empty_specs() {
        let err = Fence::parse("doc,banana").expect_err("unknown class");
        assert!(err.contains("banana"), "{err}");
        assert!(err.contains("doc"), "names the vocabulary: {err}");
        assert!(Fence::parse("").is_err(), "an empty fence is no fence");
        assert!(Fence::parse(" , ").is_err());
    }

    #[test]
    fn fenced_open_honors_standing_fence_and_cap() {
        let landscape = WishLandscape {
            proposals: vec![
                proposal(WishFacet::doc("a"), Q16::ONE),
                proposal(WishFacet::test("b_smoke"), Q16::HALF),
                proposal(WishFacet::capability("impl:c"), Q16::HALF),
                proposal(WishFacet::doc("d"), Q16::HALF),
            ],
            unmapped: vec![],
        };
        let standing = vec![
            LandscapeStanding::Open,
            LandscapeStanding::Open,
            LandscapeStanding::Open,
            LandscapeStanding::Met, // already husbanded — no chore
        ];
        let fence = Fence::parse("doc,test").unwrap();

        let chores = fenced_open(&landscape, &standing, &fence, None);
        let keys: Vec<&str> = chores.iter().map(|p| p.facet.key.as_str()).collect();
        // The capability stays outside the fence; the met doc needs nothing.
        assert_eq!(keys, vec!["a", "b_smoke"]);

        let capped = fenced_open(&landscape, &standing, &fence, Some(1));
        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].facet.key, "a", "landscape order is kept");
    }

    #[test]
    fn report_counts_render_and_serialize() {
        let landscape = WishLandscape {
            proposals: vec![
                proposal(WishFacet::doc("a"), Q16::ONE),
                proposal(WishFacet::test("b_smoke"), Q16::HALF),
            ],
            unmapped: vec![],
        };
        let standing = vec![LandscapeStanding::Open, LandscapeStanding::Open];
        let fence = Fence::parse("doc").unwrap();
        let chores = fenced_open(&landscape, &standing, &fence, None);
        let mut report = StewardReport::survey(
            Digest::of_bytes(b"ws"),
            Some(&fence),
            &landscape,
            &standing,
            &chores,
            true,
        );
        assert_eq!(report.open, 2);
        assert_eq!(report.planned, vec!["Doc a"]);
        assert!(report.is_faithful(), "no chores claimed yet");

        report.chores.push(ChoreOutcome {
            kind: "Doc".into(),
            key: "a".into(),
            wish_id: "00".into(),
            realized: false,
            iterations: 8,
        });
        assert!(!report.is_faithful(), "a failed chore is a failure");
        assert_eq!(report.husbanded(), 0);

        let text = report.render(false);
        assert!(
            text.contains("fence doc — 1 open chore(s) inside"),
            "{text}"
        );
        assert!(text.contains("husbanded: 0/1"), "{text}");
        let json = report.to_json();
        assert!(json.contains("report_id"), "{json}");
        assert!(
            !json.contains("/tmp") && !json.contains("home"),
            "durable report carries no host path: {json}"
        );
    }
}

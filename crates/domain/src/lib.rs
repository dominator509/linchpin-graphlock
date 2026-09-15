use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineStatus {
    Draft,
    Active,
    Archived,
}

#[derive(Debug, Clone)]
pub struct DomainEntity {
    pub id: EntityId,
    pub status: StateMachineStatus,
    pub created_at: SystemTime,
}

impl Default for DomainEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEntity {
    pub fn new() -> Self {
        DomainEntity {
            id: EntityId(Uuid::new_v4()),
            status: StateMachineStatus::Draft,
            created_at: SystemTime::now(),
        }
    }

    pub fn activate(&mut self) -> Result<(), &'static str> {
        if self.status != StateMachineStatus::Draft {
            return Err("Entity must be in Draft state to activate");
        }
        self.status = StateMachineStatus::Active;
        Ok(())
    }

    pub fn archive(&mut self) -> Result<(), &'static str> {
        if self.status == StateMachineStatus::Archived {
            return Err("Entity is already archived");
        }
        self.status = StateMachineStatus::Archived;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// covers: REQ-DOM-001
    #[test]
    fn test_domain_entity_lifecycle() {
        let mut entity = DomainEntity::new();
        assert_eq!(entity.status, StateMachineStatus::Draft);

        assert!(entity.activate().is_ok());
        assert_eq!(entity.status, StateMachineStatus::Active);

        assert!(entity.activate().is_err());

        assert!(entity.archive().is_ok());
        assert_eq!(entity.status, StateMachineStatus::Archived);

        assert!(entity.archive().is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorOrigin {
    HumanConception(Uuid),
    AiSuggestion(Uuid),
}

#[derive(Debug, Clone)]
pub struct ContentBlock {
    pub id: EntityId,
    pub content: String,
    pub origin: AuthorOrigin,
}

impl ContentBlock {
    pub fn new_human(content: String, user_id: Uuid) -> Self {
        ContentBlock {
            id: EntityId(Uuid::new_v4()),
            content,
            origin: AuthorOrigin::HumanConception(user_id),
        }
    }

    pub fn new_ai(content: String, model_id: Uuid) -> Self {
        ContentBlock {
            id: EntityId(Uuid::new_v4()),
            content,
            origin: AuthorOrigin::AiSuggestion(model_id),
        }
    }
}

#[cfg(test)]
mod origin_tests {
    use super::*;

    /// covers: REQ-DOM-001, REQ-DOM-002
    #[test]
    fn test_human_vs_ai_origin() {
        let human_id = Uuid::new_v4();
        let ai_id = Uuid::new_v4();

        let human_block = ContentBlock::new_human("My idea".to_string(), human_id);
        let ai_block = ContentBlock::new_ai("Suggested idea".to_string(), ai_id);

        assert!(matches!(human_block.origin, AuthorOrigin::HumanConception(id) if id == human_id));
        assert!(matches!(ai_block.origin, AuthorOrigin::AiSuggestion(id) if id == ai_id));
    }
}

#[derive(Debug, Clone)]
pub struct ScoreVector {
    pub technical_feasibility: f32,
    pub market_potential: f32,
    pub legal_risk: f32,
}

#[derive(Debug, Clone)]
pub enum UncertaintyLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct EvidenceNode {
    pub description: String,
    pub uri: String,
}

#[derive(Debug, Clone)]
pub struct OpportunityCandidate {
    pub id: EntityId,
    pub scores: ScoreVector,
    pub uncertainty: UncertaintyLevel,
    pub evidence: Vec<EvidenceNode>,
}

impl OpportunityCandidate {
    pub fn new(scores: ScoreVector, uncertainty: UncertaintyLevel) -> Self {
        OpportunityCandidate {
            id: EntityId(Uuid::new_v4()),
            scores,
            uncertainty,
            evidence: Vec::new(),
        }
    }

    pub fn add_evidence(&mut self, description: String, uri: String) {
        self.evidence.push(EvidenceNode { description, uri });
    }
}

#[cfg(test)]
mod opportunity_tests {
    use super::*;

    /// covers: REQ-DOM-003
    #[test]
    fn test_opportunity_candidate() {
        let scores = ScoreVector {
            technical_feasibility: 0.8,
            market_potential: 0.9,
            legal_risk: 0.2,
        };

        let mut candidate = OpportunityCandidate::new(scores, UncertaintyLevel::Medium);
        assert!(candidate.evidence.is_empty());

        candidate.add_evidence(
            "Prior art search".to_string(),
            "https://patents.google.com/123".to_string(),
        );
        assert_eq!(candidate.evidence.len(), 1);
        assert_eq!(candidate.evidence[0].description, "Prior art search");
    }
}

#[derive(Debug, Clone)]
pub struct Limitation {
    pub id: EntityId,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PriorArtReference {
    pub id: EntityId,
    pub citation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatMap {
    pub reference_id: EntityId,
    pub limitation_id: EntityId,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DesignAround {
    pub id: EntityId,
    pub threat_map: ThreatMap,
    pub mitigation_strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimGraphError {
    /// The limitation id is not present in this graph.
    UnknownLimitation,
    /// The prior-art reference id is not present in this graph.
    UnknownReference,
    /// A design-around was attached to a threat that is not in this graph.
    UnknownThreat,
}

impl std::fmt::Display for ClaimGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ClaimGraphError::UnknownLimitation => "limitation id is not in the claim graph",
            ClaimGraphError::UnknownReference => "reference id is not in the claim graph",
            ClaimGraphError::UnknownThreat => "threat is not in the claim graph",
        };
        write!(f, "{msg}")
    }
}

#[derive(Debug, Clone)]
pub struct ClaimGraph {
    pub limitations: Vec<Limitation>,
    pub references: Vec<PriorArtReference>,
    pub threats: Vec<ThreatMap>,
    pub design_arounds: Vec<DesignAround>,
}

impl Default for ClaimGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaimGraph {
    pub fn new() -> Self {
        ClaimGraph {
            limitations: Vec::new(),
            references: Vec::new(),
            threats: Vec::new(),
            design_arounds: Vec::new(),
        }
    }

    pub fn add_limitation(&mut self, text: String) -> EntityId {
        let id = EntityId(Uuid::new_v4());
        self.limitations.push(Limitation {
            id: id.clone(),
            text,
        });
        id
    }

    pub fn add_reference(&mut self, citation: String) -> EntityId {
        let id = EntityId(Uuid::new_v4());
        self.references.push(PriorArtReference {
            id: id.clone(),
            citation,
        });
        id
    }

    /// Map a prior-art reference onto one of this graph's limitations.
    ///
    /// REQ-DOM-005 requires the claim graph to *enforce* dependency, category
    /// and limitation identity. Both ids must already belong to this graph: a
    /// threat naming an unknown limitation or reference is rejected rather than
    /// stored, because the previous implementation accepted any `EntityId` and
    /// therefore recorded threat mappings that pointed at nothing.
    pub fn map_threat(
        &mut self,
        reference_id: EntityId,
        limitation_id: EntityId,
        description: String,
    ) -> Result<ThreatMap, ClaimGraphError> {
        if !self.limitations.iter().any(|l| l.id == limitation_id) {
            return Err(ClaimGraphError::UnknownLimitation);
        }
        if !self.references.iter().any(|r| r.id == reference_id) {
            return Err(ClaimGraphError::UnknownReference);
        }
        let threat = ThreatMap {
            reference_id,
            limitation_id,
            description,
        };
        self.threats.push(threat.clone());
        Ok(threat)
    }

    /// Attach a design-around to a threat that this graph already holds.
    ///
    /// The dependency edge is enforced for the same reason: a design-around is
    /// only meaningful relative to a recorded threat.
    pub fn add_design_around(
        &mut self,
        threat_map: ThreatMap,
        mitigation_strategy: String,
    ) -> Result<EntityId, ClaimGraphError> {
        if !self.threats.contains(&threat_map) {
            return Err(ClaimGraphError::UnknownThreat);
        }
        let id = EntityId(Uuid::new_v4());
        self.design_arounds.push(DesignAround {
            id: id.clone(),
            threat_map,
            mitigation_strategy,
        });
        Ok(id)
    }
}

#[cfg(test)]
mod claim_tests {
    use super::*;

    /// covers: REQ-DOM-004
    #[test]
    fn test_claim_graph_and_design_around() {
        let mut graph = ClaimGraph::new();

        let limit_id = graph.add_limitation("A distributed ledger".to_string());
        let ref_id = graph.add_reference("US1234567B2".to_string());

        let threat = graph
            .map_threat(ref_id, limit_id, "Reference teaches a ledger".to_string())
            .expect("both ids belong to the graph");
        assert_eq!(graph.threats.len(), 1);

        let _design_around_id = graph
            .add_design_around(threat, "Ours is non-deterministic".to_string())
            .expect("threat was recorded by this graph");
        assert_eq!(graph.design_arounds.len(), 1);
        assert_eq!(
            graph.design_arounds[0].mitigation_strategy,
            "Ours is non-deterministic"
        );
    }

    /// covers: REQ-DOM-005
    #[test]
    fn test_claim_graph_rejects_unknown_limitation_identity() {
        let mut graph = ClaimGraph::new();
        let ref_id = graph.add_reference("US1234567B2".to_string());
        let foreign_limit = EntityId(Uuid::new_v4());

        let result = graph.map_threat(ref_id, foreign_limit, "unmapped".to_string());

        assert_eq!(result, Err(ClaimGraphError::UnknownLimitation));
        assert!(
            graph.threats.is_empty(),
            "a rejected threat must not be stored: the graph recorded {}",
            graph.threats.len()
        );
    }

    /// covers: REQ-DOM-005
    #[test]
    fn test_claim_graph_rejects_unknown_reference_identity() {
        let mut graph = ClaimGraph::new();
        let limit_id = graph.add_limitation("A distributed ledger".to_string());
        let foreign_ref = EntityId(Uuid::new_v4());

        let result = graph.map_threat(foreign_ref, limit_id, "unmapped".to_string());

        assert_eq!(result, Err(ClaimGraphError::UnknownReference));
        assert!(graph.threats.is_empty());
    }

    /// covers: REQ-DOM-005
    #[test]
    fn test_design_around_requires_a_recorded_threat() {
        let mut graph = ClaimGraph::new();
        let limit_id = graph.add_limitation("A distributed ledger".to_string());
        let ref_id = graph.add_reference("US1234567B2".to_string());

        // A threat value that was never returned by map_threat: same shape,
        // none of this graph's recorded threat entries.
        let unrecorded = ThreatMap {
            reference_id: ref_id,
            limitation_id: limit_id,
            description: "never mapped".to_string(),
        };

        let result = graph.add_design_around(unrecorded, "mitigation".to_string());

        assert_eq!(result, Err(ClaimGraphError::UnknownThreat));
        assert!(graph.design_arounds.is_empty());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilingState {
    Preparation,
    Filed(String), // receipt or application number
    Commercialized,
}

#[derive(Debug, Clone)]
pub struct DocketRecord {
    pub id: EntityId,
    pub title: String,
    pub state: FilingState,
    pub public_disclosure: bool,
}

impl DocketRecord {
    pub fn new(title: String) -> Self {
        DocketRecord {
            id: EntityId(Uuid::new_v4()),
            title,
            state: FilingState::Preparation,
            public_disclosure: false,
        }
    }

    pub fn file_application(&mut self, receipt: String) -> Result<(), &'static str> {
        if self.state != FilingState::Preparation {
            return Err("Must be in preparation to file");
        }
        self.state = FilingState::Filed(receipt);
        Ok(())
    }

    pub fn mark_commercialized(&mut self) -> Result<(), &'static str> {
        match self.state {
            FilingState::Filed(_) => {
                self.state = FilingState::Commercialized;
                Ok(())
            }
            _ => Err("Must be filed before commercialization"),
        }
    }

    pub fn public_export(&mut self) -> Result<(), &'static str> {
        self.public_disclosure = true;
        Ok(())
    }
}

/// A location in the specification or drawings that supports a limitation
/// (REQ-DOM-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorKind {
    Specification,
    Figure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub kind: AnchorKind,
    /// Paragraph, figure or like reference, e.g. `[0042]` or `FIG. 3`.
    pub reference: String,
}

impl Anchor {
    pub fn specification(reference: &str) -> Result<Self, SupportMatrixError> {
        Self::new(AnchorKind::Specification, reference)
    }

    pub fn figure(reference: &str) -> Result<Self, SupportMatrixError> {
        Self::new(AnchorKind::Figure, reference)
    }

    fn new(kind: AnchorKind, reference: &str) -> Result<Self, SupportMatrixError> {
        if reference.trim().is_empty() {
            return Err(SupportMatrixError::EmptyAnchorReference);
        }
        Ok(Anchor {
            kind,
            reference: reference.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportMatrixError {
    EmptyAnchorReference,
    /// One or more exportable limitations have no specification or figure
    /// anchor. Counted rather than listed, since the caller already holds the
    /// set it passed in.
    UnsupportedLimitations(usize),
}

impl std::fmt::Display for SupportMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportMatrixError::EmptyAnchorReference => {
                write!(f, "an anchor requires a non-empty reference")
            }
            SupportMatrixError::UnsupportedLimitations(n) => write!(
                f,
                "{n} exportable limitation(s) have no specification or figure anchor"
            ),
        }
    }
}

/// Maps exportable limitations to specification/figure anchors (REQ-DOM-006).
///
/// REQ-DOM-006: "Support matrix maps every exportable limitation to spec/figure
/// anchors." The clause is a prohibition as much as a mapping: a limitation with
/// no anchor is *unsupported*, and exporting it would assert subject matter the
/// specification does not enable. `assert_exportable` is therefore the gate, and
/// it returns the shortfall rather than a boolean nobody checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SupportMatrix {
    entries: Vec<(EntityId, Anchor)>,
}

impl SupportMatrix {
    pub fn new() -> Self {
        SupportMatrix {
            entries: Vec::new(),
        }
    }

    /// Attach an anchor to a limitation. Multiple anchors per limitation are
    /// allowed: a limitation is commonly supported by both a paragraph and a
    /// figure.
    pub fn add_anchor(
        &mut self,
        limitation_id: EntityId,
        anchor: Anchor,
    ) -> Result<(), SupportMatrixError> {
        if anchor.reference.trim().is_empty() {
            return Err(SupportMatrixError::EmptyAnchorReference);
        }
        self.entries.push((limitation_id, anchor));
        Ok(())
    }

    pub fn anchors_for(&self, limitation_id: &EntityId) -> Vec<&Anchor> {
        self.entries
            .iter()
            .filter(|(id, _)| id == limitation_id)
            .map(|(_, a)| a)
            .collect()
    }

    /// Every exportable limitation that has no anchor at all.
    pub fn unsupported(&self, exportable: &[EntityId]) -> Vec<EntityId> {
        exportable
            .iter()
            .filter(|id| self.anchors_for(id).is_empty())
            .cloned()
            .collect()
    }

    /// Export gate: every exportable limitation must be anchored.
    pub fn assert_exportable(&self, exportable: &[EntityId]) -> Result<(), SupportMatrixError> {
        let missing = self.unsupported(exportable);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(SupportMatrixError::UnsupportedLimitations(missing.len()))
        }
    }

    pub fn anchor_count(&self) -> usize {
        self.entries.len()
    }
}

/// The authoritative ruleset a deadline is derived from (REQ-DOM-009).
///
/// REQ-DOM-009: "Docket deadlines require authoritative ruleset source/version."
/// Both halves are mandatory and non-empty: a source without a version cannot be
/// re-checked when the rules change, and a version without a source cannot be
/// located at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetAuthority {
    pub source: String,
    pub version: String,
}

impl RuleSetAuthority {
    pub fn new(source: &str, version: &str) -> Result<Self, DeadlineError> {
        if source.trim().is_empty() {
            return Err(DeadlineError::MissingRuleSetSource);
        }
        if version.trim().is_empty() {
            return Err(DeadlineError::MissingRuleSetVersion);
        }
        Ok(RuleSetAuthority {
            source: source.to_string(),
            version: version.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineError {
    MissingRuleSetSource,
    MissingRuleSetVersion,
    /// A model-proposed deadline cannot be authoritative without a ruleset.
    ModelSuggestionIsNotAuthoritative,
    /// A deadline cannot be authoritative while its review flag is set.
    UnreviewedDeadline,
}

impl std::fmt::Display for DeadlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            DeadlineError::MissingRuleSetSource => {
                "a deadline requires an authoritative ruleset source"
            }
            DeadlineError::MissingRuleSetVersion => {
                "a deadline requires an authoritative ruleset version"
            }
            DeadlineError::ModelSuggestionIsNotAuthoritative => {
                "a model-suggested deadline is not authoritative without a ruleset mapping"
            }
            DeadlineError::UnreviewedDeadline => "an unreviewed deadline cannot be authoritative",
        };
        write!(f, "{msg}")
    }
}

/// A docket deadline and whether it may be treated as authoritative.
///
/// REQ-PAT-004 adds the companion rule that "a model may suggest a deadline but
/// cannot make it authoritative without a ruleset/source mapping". Both are
/// enforced here, because a deadline that is authoritative merely because
/// something computed it is the same fabricated-certainty failure REQ-PAT-003
/// guards against: missing a legal date is a real-world harm, so authority must
/// be earned from a named ruleset rather than assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocketDeadline {
    /// ISO-8601 date the deadline falls on.
    pub due_date: String,
    /// Present only when the deadline is derived from a real ruleset.
    pub ruleset: Option<RuleSetAuthority>,
    /// True when a model proposed this date.
    pub suggested_by_model: bool,
    /// True when a human has reviewed the computed date.
    pub reviewed: bool,
}

impl DocketDeadline {
    /// A deadline derived from an authoritative ruleset.
    pub fn authoritative(due_date: &str, ruleset: RuleSetAuthority) -> Result<Self, DeadlineError> {
        if due_date.trim().is_empty() {
            return Err(DeadlineError::MissingRuleSetSource);
        }
        Ok(DocketDeadline {
            due_date: due_date.to_string(),
            ruleset: Some(ruleset),
            suggested_by_model: false,
            reviewed: false,
        })
    }

    /// A model-proposed date. Never authoritative on its own.
    pub fn suggested_by_model(due_date: &str) -> Self {
        DocketDeadline {
            due_date: due_date.to_string(),
            ruleset: None,
            suggested_by_model: true,
            reviewed: false,
        }
    }

    /// Attach the ruleset that makes a suggestion authoritative, after review.
    pub fn confirm_with_ruleset(&mut self, ruleset: RuleSetAuthority) -> Result<(), DeadlineError> {
        self.ruleset = Some(ruleset);
        self.reviewed = true;
        Ok(())
    }

    /// The single question the docket must answer before acting on a date.
    pub fn is_authoritative(&self) -> bool {
        if self.ruleset.is_none() {
            return false;
        }
        if self.suggested_by_model && !self.reviewed {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod docket_tests {
    use super::*;

    /// covers: REQ-DOM-007
    #[test]
    fn test_docket_state_machine() {
        let mut docket = DocketRecord::new("Novel Algorithm".to_string());
        assert_eq!(docket.state, FilingState::Preparation);

        assert!(docket.mark_commercialized().is_err());

        assert!(docket.file_application("US123456".to_string()).is_ok());
        assert!(matches!(docket.state, FilingState::Filed(ref r) if r == "US123456"));

        assert!(docket.mark_commercialized().is_ok());
        assert_eq!(docket.state, FilingState::Commercialized);

        assert!(docket.public_export().is_ok());
        assert!(docket.public_disclosure);
    }

    /// covers: REQ-DOM-009
    /// "Docket deadlines require authoritative ruleset source/version." Both
    /// halves are mandatory: a source with no version cannot be re-checked when
    /// the rules change, and a version with no source cannot be located.
    #[test]
    fn test_deadline_requires_ruleset_source_and_version() {
        assert_eq!(
            RuleSetAuthority::new("", "2026.1"),
            Err(DeadlineError::MissingRuleSetSource)
        );
        assert_eq!(
            RuleSetAuthority::new("   ", "2026.1"),
            Err(DeadlineError::MissingRuleSetSource)
        );
        assert_eq!(
            RuleSetAuthority::new("USPTO-37CFR", ""),
            Err(DeadlineError::MissingRuleSetVersion)
        );
        assert_eq!(
            RuleSetAuthority::new("USPTO-37CFR", "  "),
            Err(DeadlineError::MissingRuleSetVersion)
        );

        let authority = RuleSetAuthority::new("USPTO-37CFR", "2026.1").expect("valid ruleset");
        assert_eq!(authority.source, "USPTO-37CFR");
        assert_eq!(authority.version, "2026.1");
    }

    /// covers: REQ-DOM-009
    #[test]
    fn test_deadline_with_a_ruleset_is_authoritative() {
        let authority = RuleSetAuthority::new("USPTO-37CFR", "2026.1").unwrap();
        let deadline = DocketDeadline::authoritative("2026-11-14", authority).unwrap();
        assert!(deadline.is_authoritative());
        assert!(!deadline.suggested_by_model);
    }

    /// covers: REQ-DOM-009
    /// A model may suggest a date; it cannot make one authoritative. This is the
    /// invariant that stops a computed date being acted on as a legal deadline.
    #[test]
    fn test_model_suggestion_is_not_authoritative_until_confirmed() {
        let mut suggested = DocketDeadline::suggested_by_model("2026-11-14");
        assert!(suggested.suggested_by_model);
        assert!(
            !suggested.is_authoritative(),
            "an unconfirmed model suggestion was treated as authoritative"
        );

        // Isolate the REVIEW condition from the RULESET condition. With only the
        // assertion above, a mutation dropping the review check still passed,
        // because `ruleset.is_none()` had already forced false -- the test was
        // not discriminating for that rule. This state holds a ruleset but no
        // review, so only the review condition can reject it.
        let mut needs_review = DocketDeadline::suggested_by_model("2026-11-14");
        needs_review.ruleset = Some(RuleSetAuthority::new("USPTO-37CFR", "2026.1").unwrap());
        assert!(
            needs_review.ruleset.is_some(),
            "test premise: a ruleset is present"
        );
        assert!(
            !needs_review.is_authoritative(),
            "an unreviewed model suggestion holding a ruleset was treated as authoritative"
        );

        // Confirming with a real ruleset makes it authoritative.
        let authority = RuleSetAuthority::new("USPTO-37CFR", "2026.1").unwrap();
        suggested.confirm_with_ruleset(authority).unwrap();
        assert!(suggested.reviewed);
        assert!(suggested.is_authoritative());

        // A human-entered deadline with a ruleset is authoritative without the
        // review flag: the flag exists to gate MODEL output, not human input.
        let human = DocketDeadline::authoritative(
            "2026-11-14",
            RuleSetAuthority::new("USPTO-37CFR", "2026.1").unwrap(),
        )
        .unwrap();
        assert!(!human.suggested_by_model);
        assert!(human.is_authoritative());
    }

    /// covers: REQ-DOM-009
    #[test]
    fn test_deadline_without_a_ruleset_is_never_authoritative() {
        let mut deadline = DocketDeadline::suggested_by_model("2026-11-14");
        deadline.reviewed = true; // reviewed, but still no ruleset
        assert!(
            !deadline.is_authoritative(),
            "a reviewed deadline with no ruleset was treated as authoritative"
        );
        deadline.ruleset = None;
        assert!(!deadline.is_authoritative());
    }

    /// covers: REQ-DOM-009
    #[test]
    fn test_authoritative_deadline_rejects_an_empty_date() {
        let authority = RuleSetAuthority::new("USPTO-37CFR", "2026.1").unwrap();
        assert!(DocketDeadline::authoritative("", authority.clone()).is_err());
        assert!(DocketDeadline::authoritative("   ", authority).is_err());
    }
}

#[cfg(test)]
mod support_matrix_tests {
    use super::*;

    /// covers: REQ-DOM-006
    /// "Support matrix maps every exportable limitation to spec/figure anchors."
    #[test]
    fn test_support_matrix_maps_limitations_to_anchors() {
        let mut graph = ClaimGraph::new();
        let valve = graph.add_limitation("a self-sealing valve".to_string());
        let sensor = graph.add_limitation("a pressure sensor".to_string());

        let mut matrix = SupportMatrix::new();
        assert_eq!(matrix.anchor_count(), 0);

        matrix
            .add_anchor(valve.clone(), Anchor::specification("[0042]").unwrap())
            .unwrap();
        matrix
            .add_anchor(valve.clone(), Anchor::figure("FIG. 3").unwrap())
            .unwrap();

        let valve_anchors = matrix.anchors_for(&valve);
        assert_eq!(valve_anchors.len(), 2, "both anchors should be retained");
        assert_eq!(valve_anchors[0].kind, AnchorKind::Specification);
        assert_eq!(valve_anchors[1].kind, AnchorKind::Figure);
        assert!(matrix.anchors_for(&sensor).is_empty());
    }

    /// covers: REQ-DOM-006
    /// The prohibition half: an unanchored exportable limitation blocks export.
    #[test]
    fn test_unanchored_limitation_blocks_export() {
        let mut graph = ClaimGraph::new();
        let supported = graph.add_limitation("a self-sealing valve".to_string());
        let unsupported = graph.add_limitation("a claimed but unspecified coating".to_string());
        let exportable = vec![supported.clone(), unsupported.clone()];

        let mut matrix = SupportMatrix::new();
        matrix
            .add_anchor(supported.clone(), Anchor::specification("[0042]").unwrap())
            .unwrap();

        // The unsupported limitation is named, not merely counted.
        let missing = matrix.unsupported(&exportable);
        assert_eq!(missing, vec![unsupported.clone()]);

        // And the gate refuses, reporting how many are unsupported.
        assert_eq!(
            matrix.assert_exportable(&exportable),
            Err(SupportMatrixError::UnsupportedLimitations(1))
        );

        // Anchoring it clears the gate.
        matrix
            .add_anchor(unsupported, Anchor::figure("FIG. 7").unwrap())
            .unwrap();
        assert!(matrix.assert_exportable(&exportable).is_ok());
        assert!(matrix.unsupported(&exportable).is_empty());
    }

    /// covers: REQ-DOM-006
    #[test]
    fn test_export_with_no_limitations_is_trivially_supported() {
        let matrix = SupportMatrix::new();
        assert!(matrix.assert_exportable(&[]).is_ok());
    }

    /// covers: REQ-DOM-006
    /// An anchor with no reference points nowhere and must be rejected, both at
    /// construction and on insertion.
    #[test]
    fn test_blank_anchor_reference_is_rejected() {
        assert_eq!(
            Anchor::specification(""),
            Err(SupportMatrixError::EmptyAnchorReference)
        );
        assert_eq!(
            Anchor::figure("   "),
            Err(SupportMatrixError::EmptyAnchorReference)
        );

        let mut matrix = SupportMatrix::new();
        let blank = Anchor {
            kind: AnchorKind::Specification,
            reference: "  ".to_string(),
        };
        assert_eq!(
            matrix.add_anchor(EntityId(Uuid::new_v4()), blank),
            Err(SupportMatrixError::EmptyAnchorReference)
        );
        assert_eq!(
            matrix.anchor_count(),
            0,
            "a rejected anchor must not be stored"
        );
    }
}

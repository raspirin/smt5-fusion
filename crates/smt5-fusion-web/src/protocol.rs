use serde::{Deserialize, Serialize};

pub const MAX_FUSION_DEPTH: u32 = 4;

pub use smt5_fusion_core::model::{
    demon::{DemonContent, DemonId},
    race::{Element, Race},
    skill::{SkillCategory, SkillId},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DlcSettingsDto {
    pub konohana_sakuya: bool,
    pub dagda: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchInputDto {
    pub target: DemonId,
    pub required_skills: Vec<SkillId>,
    pub max_fusion_depth: u32,
    pub dlc: DlcSettingsDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogDto {
    pub demons: Vec<DemonCatalogDto>,
    pub skills: Vec<SkillCatalogDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemonCatalogDto {
    pub id: DemonId,
    pub race: Race,
    pub base_level: u32,
    pub content: DemonContent,
    pub initial_skills: Vec<SkillId>,
    pub natural_skills: Vec<SkillId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCatalogDto {
    pub id: SkillId,
    pub category: SkillCategory,
    pub inheritable: bool,
    pub supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultDto {
    pub session_id: u64,
    pub selection_revision: u64,
    pub input: SearchInputDto,
    pub route_count: String,
    pub actual_fusion_depth: u32,
    pub tree: Option<RouteTreeNodeDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionSnapshotDto {
    pub session_id: u64,
    pub selection_revision: u64,
    pub actual_fusion_depth: u32,
    pub tree: RouteTreeNodeDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteTreeNodeDto {
    pub path: Vec<u8>,
    pub demon: DemonId,
    pub base_level: u32,
    pub final_level: u32,
    pub estimated_macca: String,
    pub required_skills: Vec<SkillId>,
    pub upgrade_skills: Vec<UpgradeSkillDto>,
    pub acquisition: AcquisitionDto,
    pub can_change_recipe: bool,
    pub children: Vec<RouteTreeNodeDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeSkillDto {
    pub skill: SkillId,
    pub level: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionDto {
    Direct {
        summon_level: u32,
        target_level: u32,
    },
    Fusion {
        is_special: bool,
        fusion_level: u32,
        target_level: u32,
        materials: Vec<DemonId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeOptionsDto {
    pub session_id: u64,
    pub selection_revision: u64,
    pub path: Vec<u8>,
    pub demon: DemonId,
    pub options: Vec<VisibleOptionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleOptionDto {
    pub option_id: u32,
    pub selected: bool,
    pub score: u32,
    pub estimated_macca: String,
    pub acquisition: OptionAcquisitionDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionAcquisitionDto {
    Direct {
        summon_level: u32,
        target_level: u32,
    },
    Fusion {
        is_special: bool,
        route_depth: u32,
        fusion_level: u32,
        target_level: u32,
        materials: Vec<OptionMaterialDto>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionMaterialDto {
    pub demon: DemonId,
    pub required_skills: Vec<SkillId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRequest {
    Search {
        request_id: u64,
        input: SearchInputDto,
    },
    GetNodeOptions {
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
        path: Vec<u8>,
    },
    SelectNodeOption {
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
        path: Vec<u8>,
        option_id: u32,
    },
    ResetToDefault {
        request_id: u64,
        session_id: u64,
        selection_revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerResponse {
    Ready {
        catalog: CatalogDto,
    },
    SearchCompleted {
        request_id: u64,
        result: SearchResultDto,
    },
    NodeOptions {
        request_id: u64,
        options: NodeOptionsDto,
    },
    SelectionChanged {
        request_id: u64,
        snapshot: SelectionSnapshotDto,
    },
    Failure {
        request_id: Option<u64>,
        failure: WorkerFailureDto,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerFailureDto {
    pub code: WorkerFailureCode,
    pub related_id: Option<u32>,
    pub selected: Option<usize>,
    pub maximum: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerFailureCode {
    UnknownDemon,
    UnknownSkill,
    UnsupportedSkill,
    TooManySkills,
    UnavailableTarget,
    ExpandedStatesLimit,
    SkillAssignmentsLimit,
    InvalidSession,
    StaleSelection,
    InvalidPath,
    InvalidOption,
    NoRoute,
    InvalidMessage,
    Internal,
}

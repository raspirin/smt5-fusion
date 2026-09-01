use super::demon::DemonId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeMeta {
    pub result: DemonId,
    pub materials: Vec<DemonId>,
    pub is_special: bool,
}

use crate::model::skill::{Skill, SkillId};

#[derive(Debug)]
pub struct SkillCatalog {
    skills: Vec<Skill>,
}

impl SkillCatalog {
    pub(crate) fn new(mut skills: Vec<Skill>) -> Self {
        skills.sort_unstable_by_key(|skill| skill.id);
        Self { skills }
    }

    pub fn get(&self, id: SkillId) -> Option<&Skill> {
        let index = self
            .skills
            .binary_search_by_key(&id, |skill| skill.id)
            .ok()?;
        self.skills.get(index)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Skill> {
        self.skills.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::skill::{SkillCategory, SkillFlags};

    fn skill(id: u32) -> Skill {
        Skill {
            id: SkillId(id),
            category: SkillCategory::Physical,
            cost: id,
            rank: id,
            inheritable: true,
            flags: SkillFlags::default(),
        }
    }

    #[test]
    fn indexes_skills_by_id() {
        let catalog = SkillCatalog::new(vec![skill(3), skill(1), skill(2)]);

        assert_eq!(catalog.get(SkillId(2)), Some(&skill(2)));
        assert_eq!(catalog.get(SkillId(9)), None);
        assert_eq!(
            catalog.iter().map(|skill| skill.id).collect::<Vec<_>>(),
            vec![SkillId(1), SkillId(2), SkillId(3)]
        );
        assert_eq!(catalog.iter().len(), 3);
    }
}

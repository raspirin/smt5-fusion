use crate::model::{demon::DemonId, recipe::RecipeMeta};

#[derive(Debug)]
pub(crate) struct SpecialRecipeCatalog {
    recipes: Vec<RecipeMeta>,
}

impl SpecialRecipeCatalog {
    pub(crate) fn new(mut recipes: Vec<RecipeMeta>) -> Self {
        recipes.sort_unstable_by_key(|recipe| recipe.result);
        Self { recipes }
    }

    pub(crate) fn get(&self, result: DemonId) -> Option<&RecipeMeta> {
        let index = self
            .recipes
            .binary_search_by_key(&result, |recipe| recipe.result)
            .ok()?;
        self.recipes.get(index)
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &RecipeMeta> {
        self.recipes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(result: u32, materials: &[u32]) -> RecipeMeta {
        RecipeMeta {
            result: DemonId(result),
            materials: materials.iter().copied().map(DemonId).collect(),
            is_special: true,
        }
    }

    #[test]
    fn indexes_recipes_by_result() {
        let catalog = SpecialRecipeCatalog::new(vec![recipe(4, &[1, 2, 3]), recipe(3, &[1, 2])]);

        assert_eq!(catalog.get(DemonId(3)), Some(&recipe(3, &[1, 2])));
        assert_eq!(catalog.get(DemonId(9)), None);
        assert_eq!(
            catalog
                .iter()
                .map(|recipe| recipe.result)
                .collect::<Vec<_>>(),
            vec![DemonId(3), DemonId(4)]
        );
    }
}

use crate::models::{ModEntry, SortMode};

pub fn sort_mods(mods: &mut [ModEntry], mode: SortMode) {
    match mode {
        SortMode::Default => {}
        SortMode::NameAsc => mods.sort_by_key(|m| m.name.to_lowercase()),
        SortMode::NameDesc => {
            mods.sort_by_key(|m| std::cmp::Reverse(m.name.to_lowercase()));
        }
        SortMode::EnabledFirst => mods.sort_by_key(|m| !m.enabled),
        SortMode::DisabledFirst => mods.sort_by_key(|m| m.enabled),
    }
}

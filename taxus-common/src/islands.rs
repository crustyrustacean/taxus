// taxus-common/src/islands.rs

//! The single island registry (#50).
//!
//! Adding an island used to require editing three files in lockstep — the
//! `island()` template function's dispatch in the generator, an SSR render
//! helper, and the hydration match in `taxus-client` — with nothing
//! checking they agree. This module is the one list both sides compile
//! against:
//!
//! - the generator's [`crate::components`] SSR path renders each
//!   registered component and stamps `data-island="{name}"`;
//! - `taxus-client`'s `hydrate_island` hydrates by name.
//!
//! Because both sides match on [`ISLANDS`], a component missing from the
//! registry cannot be rendered at all — the failure is structural (an
//! unknown name), not a silent no-op.
//!
//! Adding an island:
//!
//! 1. Write the Yew component + props in `components/` with
//!    `#[derive(Deserialize, Serialize, Properties, PartialEq)]`.
//! 2. Register it in [`ISLANDS`] below.
//! 3. Add a `render_*` helper in the generator's pipeline (SSR) and a
//!    hydration arm in `taxus-client` — both must consult this list, so a
//!    mismatch is a name that fails loudly (`unknown island`) rather than
//!    hydration quietly disagreeing with SSR.
//!
//! The list is `const` and sorted; tests pin that it stays sorted and
//! that every name is a valid template `component=` argument (no quotes
//! or `>`).

/// One registered island.
pub struct IslandDef {
    /// The name templates pass to `island(component=…)` and the value
    /// stamped into the mount point's `data-island` attribute.
    pub name: &'static str,
}

/// Every island taxus knows, in lexical order.
///
/// This is the single source of truth for island names. Generator SSR and
/// client hydration both key off it; a name here with no hydration arm
/// hydrates nothing (and logs to the console), while a name absent here
/// never renders SSR output at all.
pub const ISLANDS: &[IslandDef] = &[
    IslandDef { name: "Counter" },
    IslandDef { name: "SearchBox" },
];

/// Construct one registry entry (re-exported at the crate root via
/// `#[macro_export]`).
#[macro_export]
macro_rules! island {
    ($name:literal) => {
        $crate::islands::IslandDef { name: $name }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn islands_are_sorted() {
        let names: Vec<_> = ISLANDS.iter().map(|i| i.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "ISLANDS must stay lexically sorted");
    }

    #[test]
    fn island_names_are_template_safe() {
        // The name is stamped into an HTML attribute and parsed back out;
        // quotes or angle brackets would break either side.
        for island in ISLANDS {
            assert!(
                !island.name.contains(['"', '\'', '>', '<']),
                "island name {:?} contains markup-unsafe characters",
                island.name
            );
        }
    }

    #[test]
    fn registry_contains_the_builtin_islands() {
        let names: Vec<_> = ISLANDS.iter().map(|i| i.name).collect();
        assert!(names.contains(&"Counter"));
        assert!(names.contains(&"SearchBox"));
    }
}

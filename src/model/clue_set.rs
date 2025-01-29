use std::collections::{BTreeMap, BTreeSet};

use log::trace;

use crate::model::{Clue, ClueOrientation, ClueWithGrouping};

#[derive(Debug, Clone, Default)]
pub struct ClueSet {
    horizontal_clues: Vec<ClueWithGrouping>,
    vertical_clues: Vec<ClueWithGrouping>,
    all_clues: Vec<ClueWithGrouping>,
}

fn assign_clue_grouping(clues: &[Clue], require_same_type: bool) -> BTreeMap<Clue, usize> {
    let mut clue_grouping: BTreeMap<Clue, usize> = BTreeMap::new();

    for (idx, clue) in clues.iter().enumerate() {
        if clue_grouping.contains_key(clue) {
            trace!(target: "clue_set", "Skipping clue {:?} because it already has a grouping", clue);
            continue;
        }

        if require_same_type {
            // For horizontal clues, only group those of the same type
            clue_grouping.insert(clue.clone(), idx);
            for other_clue in clues.iter() {
                if clue.intersects_positive(other_clue) && clue.clue_type == other_clue.clue_type {
                    clue_grouping.insert(other_clue.clone(), idx);
                }
            }
        } else {
            // For vertical clues, group all intersecting clues
            let mut clue_group = vec![clue];
            trace!(
                target: "clue_set",
                "Looking for clues matching clue group {:?}",
                clue_group
            );

            // Keep track of unprocessed clues that need to be checked
            let mut to_process: BTreeSet<&Clue> = BTreeSet::new();
            let mut processed: BTreeSet<&Clue> = BTreeSet::from([clue]);

            // Initialize with clues that intersect with the first clue
            for other_clue in clues.iter() {
                if !processed.contains(other_clue) && clue.intersects_positive(other_clue) {
                    to_process.insert(other_clue);
                }
            }

            // Process clues until no more intersections are found
            while let Some(&next_clue) = to_process.iter().next() {
                to_process.remove(&next_clue);
                processed.insert(next_clue);
                clue_group.push(next_clue);

                // Check for new intersections with unprocessed clues
                for other_clue in clues.iter() {
                    if !processed.contains(other_clue)
                        && !to_process.contains(other_clue)
                        && next_clue.intersects_positive(other_clue)
                    {
                        to_process.insert(other_clue);
                    }
                }
            }

            trace!(
                target: "clue_set",
                "Grouping is {:?}",
                clue_group
            );
            for c in clue_group.into_iter() {
                trace!(
                    target: "clue_set",
                    "Adding clue {:?} to grouping {:?}",
                    c,
                    idx
                );
                clue_grouping.insert(c.clone(), idx);
            }
        }
    }

    clue_grouping
}

fn group_clues(clues: BTreeMap<Clue, usize>) -> BTreeMap<usize, Vec<Clue>> {
    let mut groups: BTreeMap<usize, Vec<Clue>> = BTreeMap::new();
    for (clue, group) in clues.into_iter() {
        groups.entry(group).or_default().push(clue);
    }
    groups
}

/// find a merge; return the merged clue and the indices of clues to delete
fn find_mergable_clue(clues: &mut Vec<Clue>) -> Option<(Vec<Clue>, usize, usize)> {
    let mut merge_found = true;
    while merge_found {
        merge_found = false;
        for idx in 0..clues.len() {
            let clue = clues[idx].clone();
            for idx2 in idx + 1..clues.len() {
                let other_clue = clues[idx2].clone();
                let merged_clue = clue.merge(&other_clue);
                if let Some(merged_clue) = merged_clue {
                    return Some((merged_clue, idx, idx2));
                }
            }
        }
    }
    None
}

fn compress_vertical_clues(clues: &mut Vec<Clue>) {
    while let Some((merged_clue, idx1, idx2)) = find_mergable_clue(clues) {
        if idx1 > idx2 {
            clues.remove(idx1);
            clues.remove(idx2);
        } else {
            clues.remove(idx2);
            clues.remove(idx1);
        }
        clues.extend(merged_clue);
    }
}

fn remove_redundant_clues(clues: &mut Vec<Clue>) {
    let mut all_positive_assertion_rows: BTreeSet<usize> = BTreeSet::new();
    // look for all positive assertion rows
    for clue in clues.iter() {
        for assertion in clue.assertions.iter() {
            if assertion.is_positive() {
                all_positive_assertion_rows.insert(assertion.tile.row);
            }
        }
    }

    let mut clues_to_remove: Vec<usize> = Vec::new();
    // any negative assertions for this grouping are senseless, just remove them
    for (idx, clue) in clues.iter_mut().enumerate() {
        if clue.assertions.iter().any(|assertion| {
            assertion.is_negative() && all_positive_assertion_rows.contains(&assertion.tile.row)
        }) {
            match clue.without_negative_assertions() {
                Some(downgraded_clue) => *clue = downgraded_clue,
                None => clues_to_remove.push(idx),
            }
        }
    }

    for idx in clues_to_remove.into_iter().rev() {
        clues.remove(idx);
    }
}

fn sort_vert_clues(vert_clues: &mut Vec<Clue>) -> Vec<ClueWithGrouping> {
    vert_clues.sort_by(|a, b| a.sort_index.cmp(&b.sort_index));
    trace!(target: "clue_set", "before assigning clue grouping: {:?}", vert_clues);
    let clue_grouping = assign_clue_grouping(vert_clues, false);

    trace!(target: "clue_set", "Clue grouping: {:?}", clue_grouping);

    let mut clues_by_grouping = group_clues(clue_grouping);

    trace!(target: "clue_set", "Clues by grouping: {:?}", clues_by_grouping);

    clues_by_grouping.values_mut().for_each(|clues| {
        trace!(target: "clue_set", "--------------------------------");
        trace!(target: "clue_set", "before removing redundant clues:  {:?}", clues);
        remove_redundant_clues(clues);
        trace!(target: "clue_set", "after removing redundant clues:   {:?}", clues);
        compress_vertical_clues(clues);
        trace!(target: "clue_set", "after compressing vertical clues: {:?}", clues);
    });

    let mut clue_grouping: Vec<ClueWithGrouping> = vec![];

    for (group, clues) in clues_by_grouping.into_iter() {
        for clue in clues.into_iter() {
            clue_grouping.push(ClueWithGrouping {
                clue,
                group,
                orientation: ClueOrientation::Vertical,
                index: 0,
            });
        }
    }

    clue_grouping.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then(a.clue.sort_index.cmp(&b.clue.sort_index))
            .then(a.clue.assertions[0].tile.cmp(&b.clue.assertions[0].tile))
    });

    for (idx, clue_grouping) in clue_grouping.iter_mut().enumerate() {
        clue_grouping.index = idx;
    }

    trace!(target: "clue_set", "after sorting vertical clues: {:?}", clue_grouping);
    clue_grouping
}

fn sort_horiz_clues(horiz_clues: &mut Vec<Clue>) -> Vec<ClueWithGrouping> {
    horiz_clues.sort_by(|a, b| a.sort_index.cmp(&b.sort_index));
    let clue_grouping = assign_clue_grouping(horiz_clues, true);
    let mut clue_grouping: Vec<ClueWithGrouping> = clue_grouping
        .into_iter()
        .map(|(clue, group)| ClueWithGrouping {
            clue,
            group,
            orientation: ClueOrientation::Horizontal,
            index: 0,
        })
        .collect();

    clue_grouping.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then(a.clue.assertions[0].tile.cmp(&b.clue.assertions[0].tile))
    });

    for (idx, clue_grouping) in clue_grouping.iter_mut().enumerate() {
        clue_grouping.index = idx;
    }

    clue_grouping
}

impl ClueSet {
    pub fn new(clues: Vec<Clue>) -> Self {
        let mut ungrouped_horizontal_clues: Vec<Clue> = Vec::new();
        let mut ungrouped_vertical_clues: Vec<Clue> = Vec::new();

        for clue in clues.into_iter() {
            if clue.is_horizontal() {
                ungrouped_horizontal_clues.push(clue);
            } else if clue.is_vertical() {
                ungrouped_vertical_clues.push(clue);
            }
        }

        let horizontal_clues = sort_horiz_clues(&mut ungrouped_horizontal_clues);
        let vertical_clues = sort_vert_clues(&mut ungrouped_vertical_clues);
        let all_clues = horizontal_clues
            .iter()
            .chain(vertical_clues.iter())
            .cloned()
            .collect();

        Self {
            horizontal_clues,
            vertical_clues,
            all_clues,
        }
    }

    pub fn horizontal_clues(&self) -> &Vec<ClueWithGrouping> {
        &self.horizontal_clues
    }

    pub fn vertical_clues(&self) -> &Vec<ClueWithGrouping> {
        &self.vertical_clues
    }

    pub fn all_clues(&self) -> &Vec<ClueWithGrouping> {
        &self.all_clues
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Tile;

    use super::*;
    #[test]
    fn test_compress_vertical_clues() {
        let mut clues = vec![
            Clue::two_in_column(Tile::parse("0a"), Tile::parse("1a")),
            Clue::two_in_column(Tile::parse("0a"), Tile::parse("2a")),
            Clue::two_in_column(Tile::parse("0a"), Tile::parse("3a")),
        ];
        compress_vertical_clues(&mut clues);

        assert_eq!(clues.len(), 2);
        assert_eq!(
            clues[0],
            Clue::two_in_column(Tile::parse("0a"), Tile::parse("3a"))
        );
        assert_eq!(
            clues[1],
            Clue::three_in_column(Tile::parse("0a"), Tile::parse("1a"), Tile::parse("2a"))
        );
    }

    #[test]
    fn test_group_clues_no_clues_should_go_missing() {
        let mut clues = vec![
            Clue::parse_vertical("|+1f,+3c,+4b|"),
            Clue::parse_vertical("|+1a,+3f,+4c|"),
            Clue::parse_vertical("|+0a,+1b|"),
            Clue::parse_vertical("|+1a,+2d|"),
            Clue::parse_vertical("|+1d,+2e,+3e|"),
            Clue::parse_vertical("|+0c,+5c|"),
            Clue::parse_vertical("|+2c,+3c|"),
            Clue::parse_vertical("|+0f,-3b,+5a|"),
            Clue::parse_vertical("|+0a,-3f,+4e|"),
            Clue::parse_vertical("|-0e,+2e,+5c|"),
            Clue::parse_vertical("|+2b,+4e|"),
            Clue::parse_vertical("|+0e,+4f|"),
        ];
        clues.sort();

        let clue_grouping = assign_clue_grouping(&clues, false);
        let clues_by_grouping = group_clues(clue_grouping.clone());

        let mut grouped_clues = clues_by_grouping
            .into_values()
            .flatten()
            .collect::<Vec<_>>();
        grouped_clues.sort();
        assert_eq!(grouped_clues.len(), clues.len());
        for (idx, clue) in grouped_clues.iter().enumerate() {
            assert_eq!(clue, &clues[idx]);
        }
    }

    #[test]
    fn test_group_clues_expand_grouping() {
        let clues = vec![
            Clue::parse_vertical("|+0a,+1b|"),
            Clue::parse_vertical("|+2b,+4e|"),
            Clue::parse_vertical("|+0a,-3f,+4e|"),
        ];

        let clue_grouping = assign_clue_grouping(&clues, false);
        let clues_by_grouping = group_clues(clue_grouping.clone());

        assert_eq!(clues_by_grouping.len(), 1);
    }
}

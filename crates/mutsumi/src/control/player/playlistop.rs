use gtk::gio;
use gtk::prelude::*;

use crate::{PlaylistEntry, PlaylistItem};

pub enum PlaylistOp {
    Move { from: i64, to: i64 },
    Remove(i64),
    Insert { index: i64, url: String },
    // Fallback when the change can't be expressed
    Rebuild,
}

pub fn reconcile_store(store: &gio::ListStore, entries: Vec<PlaylistEntry>) {
    let mut i: u32 = 0;
    for entry in entries {
        let mut matched = None;
        let n = store.n_items();
        let mut j = i;
        while j < n {
            if let Some(item) = store.item(j).and_downcast::<PlaylistItem>()
                && item.filename() == entry.filename
            {
                matched = Some((j, item));
                break;
            }
            j += 1;
        }

        match matched {
            Some((pos, item)) => {
                if pos > i {
                    store.splice(i, pos - i, &[] as &[PlaylistItem]);
                }
                update_item(&item, &entry);
            }
            None => {
                let item = PlaylistItem::with_values(
                    &entry.filename,
                    &title_or_filename(&entry),
                    entry.current,
                );
                store.insert(i, &item);
            }
        }
        i += 1;
    }

    let n = store.n_items();
    if n > i {
        store.splice(i, n - i, &[] as &[PlaylistItem]);
    }
}

pub fn title_or_filename(entry: &PlaylistEntry) -> String {
    if entry.title.is_empty() {
        entry.filename.clone()
    } else {
        entry.title.clone()
    }
}

pub fn update_item(item: &PlaylistItem, entry: &PlaylistEntry) {
    let title = title_or_filename(entry);
    if item.title() != title {
        item.set_title(title);
    }
    if item.current() != entry.current {
        item.set_current(entry.current);
    }
}

pub fn diff_playlist(old: &[String], new: &[String]) -> Vec<PlaylistOp> {
    if old == new {
        return Vec::new();
    }

    if let Some(indices) = added_indices(old, new) {
        return indices
            .into_iter()
            .map(|index| PlaylistOp::Insert {
                index: index as i64,
                url: new[index].clone(),
            })
            .collect();
    }

    if let Some(mut indices) = added_indices(new, old) {
        indices.sort_unstable_by(|a, b| b.cmp(a));
        return indices
            .into_iter()
            .map(|index| PlaylistOp::Remove(index as i64))
            .collect();
    }

    if let Some((from, to)) = detect_move(old, new) {
        let rest_len = old.len() - 1;
        let index2 = if to >= rest_len {
            old.len()
        } else if to < from {
            to
        } else {
            to + 1
        };
        return vec![PlaylistOp::Move {
            from: from as i64,
            to: index2 as i64,
        }];
    }

    vec![PlaylistOp::Rebuild]
}

pub fn added_indices(sub: &[String], sup: &[String]) -> Option<Vec<usize>> {
    if sub.len() >= sup.len() {
        return None;
    }

    let mut added = Vec::new();
    let mut si = 0;
    for (i, item) in sup.iter().enumerate() {
        if si < sub.len() && sub[si] == *item {
            si += 1;
        } else {
            added.push(i);
        }
    }

    (si == sub.len()).then_some(added)
}

pub fn detect_move(old: &[String], new: &[String]) -> Option<(usize, usize)> {
    if old.len() != new.len() {
        return None;
    }

    for from in 0..old.len() {
        let mut without = old.to_vec();
        let moved = without.remove(from);
        for to in 0..=without.len() {
            let mut candidate = without.clone();
            candidate.insert(to, moved.clone());
            if candidate == new {
                return Some((from, to));
            }
        }
    }

    None
}

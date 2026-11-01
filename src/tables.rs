//! Non-secret display models and stable per-view sorting for compact grids.
use crate::{AppWindow, GridRow};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

pub fn row(id: i32, cells: Vec<String>) -> GridRow {
    GridRow {
        id,
        cells: ModelRc::new(VecModel::from(
            cells.into_iter().map(Into::into).collect::<Vec<_>>(),
        )),
    }
}

/// Width preferences use only displayed metadata, with bounded long values.
/// Narrow numeric ports should not take the same space as host descriptions.
pub fn inventory_widths(rows: &[GridRow], kind: i32) -> ModelRc<f32> {
    let minimum = if kind == 0 {
        [18, 22, 7, 13, 8, 18, 12]
    } else {
        [22, 22, 18, 16, 12, 12, 12]
    };
    let values = minimum
        .into_iter()
        .enumerate()
        .map(|(index, floor)| {
            rows.iter()
                .filter_map(|row| row.cells.row_data(index))
                .map(|value| value.chars().count())
                .max()
                .unwrap_or(0)
                .clamp(floor, if index < 2 { 40 } else { 24 }) as f32
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(values))
}

pub fn sorted(mut rows: Vec<GridRow>, mode: i32) -> ModelRc<GridRow> {
    if mode >= 0 {
        let column = (mode / 2) as usize;
        rows.sort_by(|a, b| {
            let a_value = a.cells.row_data(column).unwrap_or_default();
            let b_value = b.cells.row_data(column).unwrap_or_default();
            let order = match (a_value.parse::<u32>(), b_value.parse::<u32>()) {
                (Ok(a), Ok(b)) => a.cmp(&b),
                _ => a_value.to_lowercase().cmp(&b_value.to_lowercase()),
            };
            (if mode % 2 == 0 {
                order
            } else {
                order.reverse()
            })
            .then(a.id.cmp(&b.id))
        });
    }
    ModelRc::new(VecModel::from(rows))
}

pub fn setup(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_grid_sort(move |kind, mode| {
        let Some(w) = weak.upgrade() else { return };
        if !(0..18).contains(&mode) {
            return;
        }
        match kind {
            0 => {
                w.set_business_grid_sort(mode);
                w.set_business_grid(sorted(w.get_business_grid().iter().collect(), mode));
            }
            1 | 2 => {
                if kind == 1 {
                    w.set_service_grid_sort(mode);
                } else {
                    w.set_domain_grid_sort(mode);
                }
                w.set_inventory_grid(sorted(w.get_inventory_grid().iter().collect(), mode));
            }
            3 => {
                w.set_work_grid_sort(mode);
                w.set_work_grid(sorted(w.get_work_grid().iter().collect(), mode));
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_widths_favor_long_names_and_bound_extreme_values() {
        let rows = vec![row(
            1,
            vec![
                "Orders PostgreSQL primary".into(),
                "a".repeat(500),
                "5432".into(),
            ],
        )];
        let widths = inventory_widths(&rows, 0);
        assert!(widths.row_data(0).unwrap() > widths.row_data(2).unwrap());
        assert_eq!(widths.row_data(1), Some(40.0));
        assert_eq!(widths.row_count(), 7);
        assert_eq!(inventory_widths(&[], 1).row_data(4), Some(12.0));
    }

    #[test]
    fn numeric_ports_and_stable_case_insensitive_sorting() {
        let rows = vec![
            row(3, vec!["ssh".into(), "22".into()]),
            row(2, vec!["API".into(), "443".into()]),
            row(1, vec!["api".into(), "80".into()]),
        ];
        assert_eq!(
            sorted(rows.clone(), 2)
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            [3, 1, 2]
        );
        assert_eq!(
            sorted(rows.clone(), 3)
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            [2, 1, 3]
        );
        assert_eq!(
            sorted(rows, 0).iter().map(|r| r.id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }
}

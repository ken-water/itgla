mod domain;

use std::{cell::RefCell, rc::Rc};

use domain::{Asset, Health, PROJECTS, ResourceKind, filter_assets, kind_from_label};
use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

#[derive(Default)]
struct UiState {
    project_index: usize,
    filter: Option<ResourceKind>,
    query: String,
}

fn to_row(asset: &Asset) -> AssetRow {
    AssetRow {
        kind: SharedString::from(asset.kind.label()),
        mark: SharedString::from(asset.kind.mark()),
        name: SharedString::from(asset.name),
        detail: SharedString::from(asset.detail),
        status: SharedString::from(asset.health.label()),
        status_detail: SharedString::from(asset.status_detail),
        environment: SharedString::from(asset.environment),
        tone: match asset.health {
            Health::Healthy => 0,
            Health::Warning => 1,
            Health::Critical => 2,
        },
        kind_tone: match asset.kind {
            ResourceKind::Website => 0,
            ResourceKind::Domain => 1,
            ResourceKind::Certificate => 2,
            ResourceKind::Server => 3,
            ResourceKind::Service => 4,
        },
    }
}

fn refresh(window: &AppWindow, state: &UiState) {
    let project = PROJECTS[state.project_index];
    let assets: Vec<AssetRow> = filter_assets(project, state.filter, &state.query)
        .into_iter()
        .map(to_row)
        .collect();
    let asset_count = assets.len() as i32;
    window.set_selected_project(SharedString::from(project));
    window.set_assets(ModelRc::new(VecModel::from(assets)));
    window.set_result_count(asset_count);
    window.set_selected_asset(-1);
}

fn main() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(UiState::default()));
    refresh(&window, &state.borrow());

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    window.on_project_selected(move |index| {
        let Some(window) = weak.upgrade() else { return };
        let mut state = callback_state.borrow_mut();
        state.project_index = (index as usize).min(PROJECTS.len() - 1);
        refresh(&window, &state);
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    window.on_filter_selected(move |label| {
        let Some(window) = weak.upgrade() else { return };
        let mut state = callback_state.borrow_mut();
        state.filter = if label == "全部" {
            None
        } else {
            kind_from_label(&label)
        };
        refresh(&window, &state);
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    window.on_search_changed(move |query| {
        let Some(window) = weak.upgrade() else { return };
        let mut state = callback_state.borrow_mut();
        state.query = query.to_string();
        refresh(&window, &state);
    });

    window.run()
}

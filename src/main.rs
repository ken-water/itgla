mod domain;
mod storage;

use std::{cell::RefCell, rc::Rc};

use domain::{Asset, Health, Project, ResourceKind, filter_assets, kind_from_label};
use slint::{ModelRc, SharedString, VecModel};
use storage::{Repository, StorageError};

slint::include_modules!();

#[derive(Default)]
struct UiState {
    project_index: usize,
    filter: Option<ResourceKind>,
    query: String,
    projects: Vec<Project>,
    assets: Vec<Asset>,
}

fn to_row(asset: &Asset) -> AssetRow {
    AssetRow {
        kind: SharedString::from(asset.kind.label()),
        mark: SharedString::from(asset.kind.mark()),
        name: SharedString::from(asset.name.as_str()),
        detail: SharedString::from(asset.detail.as_str()),
        status: SharedString::from(asset.health.label()),
        status_detail: SharedString::from(asset.status_detail.as_str()),
        environment: SharedString::from(asset.environment.as_str()),
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

fn refresh(
    window: &AppWindow,
    state: &mut UiState,
    repository: &Repository,
) -> Result<(), StorageError> {
    let Some(project) = state.projects.get(state.project_index) else {
        window.set_assets(ModelRc::new(VecModel::<AssetRow>::default()));
        window.set_result_count(0);
        return Ok(());
    };
    state.assets = repository.assets_for_project(project.id)?;
    let assets: Vec<AssetRow> = filter_assets(&state.assets, state.filter, &state.query)
        .into_iter()
        .map(to_row)
        .collect();
    let asset_count = assets.len() as i32;
    window.set_selected_project(SharedString::from(project.name.as_str()));
    window.set_assets(ModelRc::new(VecModel::from(assets)));
    window.set_result_count(asset_count);
    window.set_selected_asset(-1);
    window.set_system_error(SharedString::new());
    Ok(())
}

fn show_storage_error(window: &AppWindow, error: &StorageError) {
    window.set_system_error(SharedString::from(format!("无法读取本地数据：{error}")));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Rc::new(Repository::open_default()?);
    let projects = repository.projects()?;
    let window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(UiState {
        projects,
        ..UiState::default()
    }));
    refresh(&window, &mut state.borrow_mut(), &repository)?;

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_project_selected(move |index| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        state.project_index = (index as usize).min(state.projects.len().saturating_sub(1));
        if let Err(error) = refresh(&window, &mut state, &callback_repository) {
            show_storage_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_filter_selected(move |label| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        state.filter = if label == "全部" {
            None
        } else {
            kind_from_label(&label)
        };
        if let Err(error) = refresh(&window, &mut state, &callback_repository) {
            show_storage_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_search_changed(move |query| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        state.query = query.to_string();
        if let Err(error) = refresh(&window, &mut state, &callback_repository) {
            show_storage_error(&window, &error);
        }
    });

    window.run()?;
    Ok(())
}

mod domain;
mod storage;

use std::{cell::RefCell, rc::Rc};

use domain::{
    Asset, AssetDraft, Health, Project, ResourceKind, filter_assets, kind_from_label, parse_tags,
};
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

fn to_asset_row(asset: &Asset) -> AssetRow {
    let detail = if asset.tags.is_empty() {
        asset.detail.clone()
    } else {
        format!("{} · #{}", asset.detail, asset.tags.join("  #"))
    };
    AssetRow {
        id: i32::try_from(asset.id).map_or(i32::MAX, |value| value),
        kind: SharedString::from(asset.kind.label()),
        mark: SharedString::from(asset.kind.mark()),
        name: SharedString::from(asset.name.as_str()),
        detail: SharedString::from(detail),
        status: SharedString::from(asset.health.label()),
        status_detail: SharedString::from(asset.status_detail.as_str()),
        environment: SharedString::from(asset.environment.as_str()),
        tone: asset.health.index(),
        kind_tone: asset.kind.index(),
        kind_index: asset.kind.index(),
        health_index: asset.health.index(),
        tags: SharedString::from(asset.tags.join(", ")),
    }
}

fn project_initials(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    if initials.chars().count() == 1 {
        name.chars().take(2).flat_map(char::to_uppercase).collect()
    } else {
        initials
    }
}

fn to_project_row(project: &Project) -> ProjectRow {
    let summary = if project.attention_count == 0 {
        format!("{} 项资源 · 状态正常", project.asset_count)
    } else {
        format!(
            "{} 项资源 · {} 项关注",
            project.asset_count, project.attention_count
        )
    };
    ProjectRow {
        id: i32::try_from(project.id).map_or(i32::MAX, |value| value),
        name: SharedString::from(project.name.as_str()),
        short_name: SharedString::from(project_initials(&project.name)),
        summary: SharedString::from(summary),
    }
}

fn refresh(
    window: &AppWindow,
    state: &mut UiState,
    repository: &Repository,
) -> Result<(), StorageError> {
    state.projects = repository.projects()?;
    if state.projects.is_empty() {
        state.assets.clear();
        window.set_projects(ModelRc::new(VecModel::<ProjectRow>::default()));
        window.set_assets(ModelRc::new(VecModel::<AssetRow>::default()));
        window.set_result_count(0);
        return Ok(());
    }
    state.project_index = state.project_index.min(state.projects.len() - 1);
    let project = &state.projects[state.project_index];
    state.assets = repository.assets_for_project(project.id)?;
    let assets: Vec<AssetRow> = filter_assets(&state.assets, state.filter, &state.query)
        .into_iter()
        .map(to_asset_row)
        .collect();
    window.set_projects(ModelRc::new(VecModel::from(
        state
            .projects
            .iter()
            .map(to_project_row)
            .collect::<Vec<_>>(),
    )));
    window.set_result_count(assets.len() as i32);
    window.set_assets(ModelRc::new(VecModel::from(assets)));
    window.set_selected_project(SharedString::from(project.name.as_str()));
    window.set_selected_project_index(state.project_index as i32);
    window.set_selected_asset(-1);
    window.set_system_error(SharedString::new());
    Ok(())
}

fn show_error(window: &AppWindow, error: &dyn std::fmt::Display) {
    window.set_system_error(SharedString::from(error.to_string()));
    window.set_system_status(SharedString::new());
}

fn show_success(window: &AppWindow, message: &str) {
    window.set_system_error(SharedString::new());
    window.set_system_status(SharedString::from(message));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Rc::new(RefCell::new(Repository::open_default()?));
    let window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(UiState::default()));
    refresh(&window, &mut state.borrow_mut(), &repository.borrow())?;

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_project_selected(move |index| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        state.project_index = index.max(0) as usize;
        if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
            show_error(&window, &error);
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
        if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
            show_error(&window, &error);
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
        if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_save_resource(
        move |id, kind, name, detail, status_detail, environment, health, tags| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let mut state = callback_state.borrow_mut();
            let Some(project) = state.projects.get(state.project_index) else {
                return;
            };
            let tags = match parse_tags(&tags) {
                Ok(tags) => tags,
                Err(error) => {
                    show_error(&window, &error);
                    return;
                }
            };
            let draft = AssetDraft {
                project_id: project.id,
                kind: ResourceKind::from_index(kind),
                name: name.to_string(),
                detail: detail.to_string(),
                status_detail: status_detail.to_string(),
                environment: environment.to_string(),
                health: Health::from_index(health),
                tags,
            };
            let saved = callback_repository
                .borrow_mut()
                .save_asset((id != 0).then_some(i64::from(id)), &draft);
            match saved {
                Ok(_) => {
                    if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow())
                    {
                        show_error(&window, &error);
                        return;
                    }
                    window.set_resource_editor_open(false);
                    show_success(&window, "资源已保存到本地");
                }
                Err(error) => show_error(&window, &error),
            }
        },
    );

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_archive_resource(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let archived = callback_repository.borrow().archive_asset(i64::from(id));
        match archived {
            Ok(()) => {
                let mut state = callback_state.borrow_mut();
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_archive_confirm_open(false);
                window.set_resource_editor_open(false);
                show_success(&window, "资源已归档");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_save_project(move |id, name| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let saved = callback_repository
            .borrow_mut()
            .save_project((id != 0).then_some(i64::from(id)), &name);
        match saved {
            Ok(saved_id) => {
                let mut state = callback_state.borrow_mut();
                match callback_repository.borrow().projects() {
                    Ok(projects) => {
                        state.project_index = projects
                            .iter()
                            .position(|project| project.id == saved_id)
                            .map_or(0, |index| index);
                    }
                    Err(error) => {
                        show_error(&window, &error);
                        return;
                    }
                }
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_project_editor_open(false);
                show_success(&window, "项目已保存到本地");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_archive_project(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let archived = callback_repository
            .borrow_mut()
            .archive_project(i64::from(id));
        match archived {
            Ok(()) => {
                let mut state = callback_state.borrow_mut();
                state.project_index = 0;
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_archive_confirm_open(false);
                window.set_project_editor_open(false);
                show_success(&window, "项目及其资源已归档");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    window.run()?;
    Ok(())
}

mod domain;
mod storage;

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use domain::{
    Asset, AssetDraft, GlobalAsset, Health, Project, Relationship, RelationshipKind, ResourceKind,
    filter_assets, kind_from_label, parse_tags,
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
    relationships: Vec<Relationship>,
}

fn graph_models(
    assets: &[Asset],
    relationships: &[Relationship],
) -> (Vec<GraphNodeRow>, Vec<GraphEdgeRow>) {
    let positions = graph_positions(assets.len().min(8));
    let visible = assets.iter().take(positions.len()).collect::<Vec<_>>();
    let nodes = visible
        .iter()
        .zip(&positions)
        .map(|(asset, &(x, y))| GraphNodeRow {
            id: i32::try_from(asset.id).map_or(i32::MAX, |value| value),
            title: SharedString::from(asset.name.as_str()),
            subtitle: SharedString::from(asset.kind.label()),
            mark: SharedString::from(asset.kind.mark()),
            kind_tone: asset.kind.index(),
            x,
            y,
        })
        .collect();
    let edges = relationships
        .iter()
        .filter_map(|relationship| {
            let source = visible
                .iter()
                .position(|asset| asset.id == relationship.source_asset_id)?;
            let target = visible
                .iter()
                .position(|asset| asset.id == relationship.target_asset_id)?;
            let (source_x, source_y) = positions[source];
            let (target_x, target_y) = positions[target];
            Some(GraphEdgeRow {
                path: SharedString::from(format!(
                    "M {} {} L {} {}",
                    source_x + 58.0,
                    source_y + 26.0,
                    target_x + 58.0,
                    target_y + 26.0
                )),
            })
        })
        .collect();
    (nodes, edges)
}

fn graph_positions(count: usize) -> Vec<(f32, f32)> {
    match count {
        0 => vec![],
        1 => vec![(137.0, 150.0)],
        2 => vec![(54.0, 150.0), (220.0, 150.0)],
        3 => vec![(137.0, 45.0), (220.0, 225.0), (54.0, 225.0)],
        4 => vec![(45.0, 65.0), (229.0, 65.0), (229.0, 235.0), (45.0, 235.0)],
        5 => vec![
            (137.0, 25.0),
            (245.0, 105.0),
            (205.0, 245.0),
            (69.0, 245.0),
            (29.0, 105.0),
        ],
        6 => vec![
            (137.0, 18.0),
            (245.0, 75.0),
            (245.0, 225.0),
            (137.0, 282.0),
            (29.0, 225.0),
            (29.0, 75.0),
        ],
        7 => vec![
            (137.0, 15.0),
            (245.0, 60.0),
            (245.0, 165.0),
            (205.0, 270.0),
            (69.0, 270.0),
            (29.0, 165.0),
            (29.0, 60.0),
        ],
        _ => vec![
            (137.0, 10.0),
            (245.0, 55.0),
            (245.0, 150.0),
            (245.0, 250.0),
            (137.0, 288.0),
            (29.0, 250.0),
            (29.0, 150.0),
            (29.0, 55.0),
        ],
    }
}

fn to_relationship_row(relationship: &Relationship) -> RelationshipRow {
    RelationshipRow {
        id: i32::try_from(relationship.id).map_or(i32::MAX, |value| value),
        source: SharedString::from(relationship.source_name.as_str()),
        kind: SharedString::from(relationship.kind.label()),
        target: SharedString::from(relationship.target_name.as_str()),
    }
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

fn to_global_result(result: &GlobalAsset, projects: &[Project]) -> GlobalResultRow {
    let project_index = projects
        .iter()
        .position(|project| project.id == result.asset.project_id)
        .map_or(0, |index| index as i32);
    GlobalResultRow {
        asset_id: i32::try_from(result.asset.id).map_or(i32::MAX, |value| value),
        project_index,
        name: SharedString::from(result.asset.name.as_str()),
        project: SharedString::from(result.project_name.as_str()),
        kind: SharedString::from(result.asset.kind.label()),
        mark: SharedString::from(result.asset.kind.mark()),
        detail: SharedString::from(result.asset.detail.as_str()),
        status: SharedString::from(result.asset.health.label()),
        tone: result.asset.health.index(),
        kind_tone: result.asset.kind.index(),
    }
}

fn refresh_global_results(
    window: &AppWindow,
    state: &UiState,
    repository: &Repository,
    query: &str,
    attention_only: bool,
) -> Result<(), StorageError> {
    let results = repository
        .search_assets(query, attention_only)?
        .iter()
        .map(|result| to_global_result(result, &state.projects))
        .collect::<Vec<_>>();
    window.set_global_result_count(results.len() as i32);
    window.set_global_results(ModelRc::new(VecModel::from(results)));
    Ok(())
}

fn refresh(
    window: &AppWindow,
    state: &mut UiState,
    repository: &Repository,
) -> Result<(), StorageError> {
    state.projects = repository.projects()?;
    if state.projects.is_empty() {
        state.assets.clear();
        state.relationships.clear();
        window.set_projects(ModelRc::new(VecModel::<ProjectRow>::default()));
        window.set_assets(ModelRc::new(VecModel::<AssetRow>::default()));
        window.set_graph_nodes(ModelRc::new(VecModel::<GraphNodeRow>::default()));
        window.set_graph_edges(ModelRc::new(VecModel::<GraphEdgeRow>::default()));
        window.set_relationships(ModelRc::new(VecModel::<RelationshipRow>::default()));
        window.set_asset_options(ModelRc::new(VecModel::<SharedString>::default()));
        window.set_project_asset_count(0);
        window.set_result_count(0);
        return Ok(());
    }
    state.project_index = state.project_index.min(state.projects.len() - 1);
    let project = &state.projects[state.project_index];
    state.assets = repository.assets_for_project(project.id)?;
    state.relationships = repository.relationships_for_project(project.id)?;
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
    window.set_project_asset_count(state.assets.len() as i32);
    window.set_assets(ModelRc::new(VecModel::from(assets)));
    let (graph_nodes, graph_edges) = graph_models(&state.assets, &state.relationships);
    window.set_graph_nodes(ModelRc::new(VecModel::from(graph_nodes)));
    window.set_graph_edges(ModelRc::new(VecModel::from(graph_edges)));
    window.set_relationships(ModelRc::new(VecModel::from(
        state
            .relationships
            .iter()
            .map(to_relationship_row)
            .collect::<Vec<_>>(),
    )));
    window.set_asset_options(ModelRc::new(VecModel::from(
        state
            .assets
            .iter()
            .map(|asset| SharedString::from(format!("{} · {}", asset.kind.label(), asset.name)))
            .collect::<Vec<_>>(),
    )));
    let selected_graph_id = state.assets.first().map_or(0, |asset| {
        i32::try_from(asset.id).map_or(i32::MAX, |value| value)
    });
    window.set_selected_graph_asset_id(selected_graph_id);
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
    let (json_path, backup_path) = Repository::default_portability_paths()?;
    window.set_json_exchange_path(SharedString::from(json_path.to_string_lossy().into_owned()));
    window.set_database_backup_path(SharedString::from(
        backup_path.to_string_lossy().into_owned(),
    ));
    refresh(&window, &mut state.borrow_mut(), &repository.borrow())?;
    refresh_global_results(&window, &state.borrow(), &repository.borrow(), "", false)?;

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

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_save_relationship(move |source_index, target_index, kind_index| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        let Some(project) = state.projects.get(state.project_index) else {
            return;
        };
        let Some(source) = state.assets.get(source_index.max(0) as usize) else {
            show_error(&window, &"请选择来源资源");
            return;
        };
        let Some(target) = state.assets.get(target_index.max(0) as usize) else {
            show_error(&window, &"请选择目标资源");
            return;
        };
        let saved = callback_repository.borrow().save_relationship(
            project.id,
            source.id,
            target.id,
            RelationshipKind::from_index(kind_index),
        );
        match saved {
            Ok(_) => {
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_relationship_editor_open(false);
                show_success(&window, "关系已添加到图谱");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_archive_relationship(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let archived = callback_repository
            .borrow()
            .archive_relationship(i64::from(id));
        match archived {
            Ok(()) => {
                let mut state = callback_state.borrow_mut();
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_archive_confirm_open(false);
                show_success(&window, "关系已从图谱移除");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    window.on_graph_node_selected(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let state = callback_state.borrow();
        let visible = filter_assets(&state.assets, state.filter, &state.query);
        let index = visible
            .iter()
            .position(|asset| asset.id == i64::from(id))
            .map_or(-1, |index| index as i32);
        window.set_selected_asset(index);
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_global_search_changed(move |query, attention_only| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Err(error) = refresh_global_results(
            &window,
            &callback_state.borrow(),
            &callback_repository.borrow(),
            &query,
            attention_only,
        ) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_global_result_selected(move |project_index, asset_id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut state = callback_state.borrow_mut();
        state.project_index = project_index.max(0) as usize;
        state.filter = None;
        state.query.clear();
        if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
            show_error(&window, &error);
            return;
        }
        let selected = state
            .assets
            .iter()
            .position(|asset| asset.id == i64::from(asset_id))
            .map_or(-1, |index| index as i32);
        window.set_active_filter(SharedString::from("全部"));
        window.set_project_query(SharedString::new());
        window.set_selected_asset(selected);
        window.set_selected_graph_asset_id(asset_id);
        window.set_global_search_open(false);
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    window.on_export_json(move |path| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(path.to_string());
        match callback_repository.borrow().export_json(&path) {
            Ok(()) => show_success(&window, "JSON 已导出到指定路径"),
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_import_json(move |path| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(path.to_string());
        let imported = callback_repository.borrow_mut().import_json(&path);
        match imported {
            Ok(()) => {
                let mut state = callback_state.borrow_mut();
                state.project_index = 0;
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_data_confirm_open(false);
                show_success(&window, "JSON 已导入，导入前恢复点已创建");
            }
            Err(error) => {
                window.set_data_confirm_open(false);
                show_error(&window, &error);
            }
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    window.on_create_backup(move |path| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(path.to_string());
        match callback_repository.borrow().create_backup(&path) {
            Ok(()) => show_success(&window, "完整数据库备份已创建"),
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_state = Rc::clone(&state);
    let callback_repository = Rc::clone(&repository);
    window.on_restore_backup(move |path| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(path.to_string());
        let restored = callback_repository.borrow_mut().restore_backup(&path);
        match restored {
            Ok(()) => {
                let mut state = callback_state.borrow_mut();
                state.project_index = 0;
                if let Err(error) = refresh(&window, &mut state, &callback_repository.borrow()) {
                    show_error(&window, &error);
                    return;
                }
                window.set_data_confirm_open(false);
                show_success(&window, "数据库已恢复，恢复前快照已保留");
            }
            Err(error) => {
                window.set_data_confirm_open(false);
                show_error(&window, &error);
            }
        }
    });

    window.run()?;
    Ok(())
}

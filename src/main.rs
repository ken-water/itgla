#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[allow(dead_code)]
mod domain;
mod import_data;
mod operational;
mod secrets;
#[allow(dead_code)]
mod storage;

use std::{cell::RefCell, cmp::Ordering, fs, net::IpAddr, path::PathBuf, rc::Rc};

use domain::{
    ServerColumn, ServerDraft, ServerRecord, SiteDraft, SiteRecord, WorkDraft, WorkItem, WorkKind,
    WorkStatus, server_tags_from_input,
};
use import_data::{TabularData, read_tabular};
#[cfg(debug_assertions)]
use slint::{ComponentHandle, PhysicalSize};
use slint::{Model, ModelRc, SharedString, VecModel};
use storage::{
    ImportDuplicatePolicy, Repository, ServerImportDefaults, ServerImportTarget,
    SiteImportDefaults, SiteImportTarget, StorageError, WorkspacePreferences,
};

slint::include_modules!();

#[derive(Default)]
struct TableState {
    query: String,
    tag: String,
    sort_mode: usize,
    show_hidden: bool,
}

#[derive(Default)]
struct SiteTableState {
    query: String,
    sort_mode: usize,
    show_hidden: bool,
}

#[derive(Default)]
struct ImportSession {
    data: Option<TabularData>,
    mappings: Vec<usize>,
    existing_columns: Vec<ServerColumn>,
}

#[derive(Default)]
struct SiteImportSession {
    data: Option<TabularData>,
    mappings: Vec<usize>,
}

#[derive(Default)]
struct WorkListState {
    query: String,
    kind_filter: i32,
    status_filter: i32,
    sort_mode: i32,
}

const SUGGESTED_CUSTOM_FIELDS: &[&str] = &[
    "Notes",
    "Cloud Provider",
    "Region",
    "OS",
    "Project",
    "Owner",
    "Service",
    "Domain",
    "Panel URL",
    "Login URL",
];

const IMPORT_TEMPLATE_CSV: &str = concat!(
    "Name,IP Address,Ports,Environment,Status,Purpose,Tags,Notes\n",
    "api-gateway-01,192.0.2.10,\"22,80,443\",production,active,\"Public gateway server\",\"api,customer-facing\",\"Primary ingress\"\n",
    "postgres-01,198.51.100.20,5432,production,active,\"Primary PostgreSQL server\",database,\"Daily backups\"\n",
);

const SITE_IMPORT_TEMPLATE_CSV: &str = concat!(
    "Name,URL,Environment,Status,Purpose,Tags\n",
    "Customer portal,https://app.example.com,production,active,Customer application,customer-facing\n",
    "Status page,https://status.example.com,production,active,Public service status,operations\n",
);

fn to_row(server: &ServerRecord) -> ServerRow {
    ServerRow {
        id: i32::try_from(server.id).unwrap_or(i32::MAX),
        name: server.name.as_str().into(),
        tags: server.tags.join(", ").into(),
        ip_address: server.ip_address.as_str().into(),
        ports: server.ports.as_str().into(),
        environment: server.environment.as_str().into(),
        status: server.status.as_str().into(),
        purpose: server.purpose.as_str().into(),
        custom_values: ModelRc::new(VecModel::from(
            server
                .custom_values
                .iter()
                .map(SharedString::from)
                .collect::<Vec<_>>(),
        )),
    }
}

fn site_to_row(site: &SiteRecord) -> SiteRow {
    SiteRow {
        id: i32::try_from(site.id).unwrap_or(i32::MAX),
        name: site.name.as_str().into(),
        url: site.url.as_str().into(),
        environment: site.environment.as_str().into(),
        status: site.status.as_str().into(),
        purpose: site.purpose.as_str().into(),
        tags: site.tags.join(", ").into(),
    }
}

fn work_to_row(work: &WorkItem) -> WorkRow {
    WorkRow {
        id: i32::try_from(work.id).unwrap_or(i32::MAX),
        title: work.title.as_str().into(),
        kind: work.kind.label().into(),
        status: work.status.label().into(),
        priority: work.priority.as_str().into(),
        owner: work.owner.as_str().into(),
        due_at: work.due_at.as_str().into(),
        due_state: work.due_state.label().into(),
        checklist: format!(
            "{}/{} checked",
            work.checklist_completed, work.checklist_total
        )
        .into(),
        scope: format!(
            "{}P · {}S · {}A",
            work.project_count, work.server_count, work.asset_count
        )
        .into(),
    }
}

fn set_work_rows(
    window: &AppWindow,
    repository: &Repository,
    state: &WorkListState,
) -> Result<(), StorageError> {
    let query = state.query.trim().to_lowercase();
    let mut work_items = repository
        .work_items()?
        .into_iter()
        .filter(|work| {
            (state.kind_filter == 0 || work.kind.index() + 1 == state.kind_filter)
                && (state.status_filter == 0 || work.status.index() + 1 == state.status_filter)
                && (query.is_empty()
                    || work.title.to_lowercase().contains(&query)
                    || work.owner.to_lowercase().contains(&query)
                    || work.notes.to_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    match state.sort_mode {
        1 => work_items.sort_by_key(|work| {
            (
                work.priority.to_lowercase(),
                work.title.to_lowercase(),
                work.id,
            )
        }),
        2 => work_items.sort_by_key(|work| {
            (
                work.due_at.trim().is_empty(),
                work.due_at.to_lowercase(),
                work.title.to_lowercase(),
                work.id,
            )
        }),
        3 => {
            work_items.sort_by_key(|work| (work.status.index(), work.title.to_lowercase(), work.id))
        }
        4 => work_items.sort_by_key(|work| (work.title.to_lowercase(), work.id)),
        _ => {}
    }
    let rows = work_items.iter().map(work_to_row).collect::<Vec<_>>();
    window.set_work_items(ModelRc::new(VecModel::from(rows)));
    Ok(())
}

fn clear_work_editor(window: &AppWindow) {
    window.set_selected_work_id(0);
    window.set_work_title(SharedString::new());
    window.set_work_kind("change".into());
    window.set_work_status("inbox".into());
    window.set_work_kind_index(0);
    window.set_work_status_index(0);
    window.set_work_priority("P2".into());
    window.set_work_owner(SharedString::new());
    window.set_work_start_at(SharedString::new());
    window.set_work_due_at(SharedString::new());
    window.set_work_notes(SharedString::new());
    window.set_work_project_count(0);
    window.set_work_server_count(0);
    window.set_work_asset_count(0);
    window.set_work_health_message("Add a project or resource scope".into());
    window.set_work_health_has_warnings(true);
    window.set_work_health_action(1);
    window.set_work_health_action_label("Review scope".into());
    window.set_work_impact_ignored(0);
    window.set_checklist_items(ModelRc::new(VecModel::default()));
    window.set_checklist_dialog_open(false);
    window.set_checklist_input(SharedString::new());
}

fn set_work_editor(window: &AppWindow, work: &WorkItem) {
    window.set_selected_work_id(i32::try_from(work.id).unwrap_or(i32::MAX));
    window.set_work_title(work.title.as_str().into());
    window.set_work_kind(work.kind.key().into());
    window.set_work_status(work.status.key().into());
    window.set_work_kind_index(work.kind.index());
    window.set_work_status_index(work.status.index());
    window.set_work_priority(work.priority.as_str().into());
    window.set_work_owner(work.owner.as_str().into());
    window.set_work_start_at(work.start_at.as_str().into());
    window.set_work_due_at(work.due_at.as_str().into());
    window.set_work_notes(work.notes.as_str().into());
    window.set_work_project_count(work.project_count as i32);
    window.set_work_server_count(work.server_count as i32);
    window.set_work_asset_count(work.asset_count as i32);
}

fn set_work_scope_models(
    window: &AppWindow,
    repository: &Repository,
    work_id: i64,
) -> Result<(), StorageError> {
    let (projects, servers) = repository.work_scope(work_id)?;
    let (impact_depth, impact_filter) = repository.work_impact_view(work_id)?;
    window.set_work_impact_depth(impact_depth as i32);
    window.set_work_impact_filter_index(impact_filter as i32);
    let assets = repository.work_asset_scope(work_id)?;
    let inferred = repository.inferred_work_assets(work_id)?;
    set_impact_summary(window, &inferred);
    window.set_work_impacts_capped(inferred.len() >= 50);
    window.set_work_projects(ModelRc::new(VecModel::from(
        projects
            .into_iter()
            .map(|item| WorkProjectChoice {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                name: item.name.into(),
                selected: item.selected,
            })
            .collect::<Vec<_>>(),
    )));
    window.set_work_servers(ModelRc::new(VecModel::from(
        servers
            .into_iter()
            .map(|item| WorkServerChoice {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                label: item.label.into(),
                selected: item.selected,
            })
            .collect::<Vec<_>>(),
    )));
    window.set_work_assets(ModelRc::new(VecModel::from(
        assets
            .into_iter()
            .map(|item| WorkAssetChoice {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                label: item.label.into(),
                selected: item.selected,
                origin: item.origin.into(),
            })
            .collect::<Vec<_>>(),
    )));
    window.set_work_impacts(ModelRc::new(VecModel::from(
        inferred
            .into_iter()
            .map(|item| WorkImpactChoice {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                label: item.label.into(),
                category: item.category.into(),
                reason: item.reason.into(),
            })
            .collect::<Vec<_>>(),
    )));
    window.set_work_project_count(
        window
            .get_work_projects()
            .iter()
            .filter(|item| item.selected)
            .count() as i32,
    );
    window.set_work_server_count(
        window
            .get_work_servers()
            .iter()
            .filter(|item| item.selected)
            .count() as i32,
    );
    window.set_work_asset_count(
        window
            .get_work_assets()
            .iter()
            .filter(|item| item.selected)
            .count() as i32,
    );
    set_work_impact_depth(window, repository, work_id, impact_depth as i32)?;
    let health = repository.work_health(work_id)?;
    window.set_work_health_message(health.message().into());
    window.set_work_health_has_warnings(!health.warnings.is_empty());
    window.set_work_health_action(health.action.map_or(0, |action| action.code()));
    window.set_work_health_action_label(health.action.map_or("", |action| action.label()).into());
    Ok(())
}

fn set_work_impact_depth(
    window: &AppWindow,
    repository: &Repository,
    work_id: i64,
    depth: i32,
) -> Result<(), StorageError> {
    let inferred = repository.inferred_work_assets_with_depth(work_id, i64::from(depth))?;
    let total = repository.inferred_work_asset_total_with_depth(work_id, i64::from(depth))?;
    window.set_work_impact_ignored(repository.excluded_impact_count(work_id)? as i32);
    set_impact_summary(window, &inferred);
    window.set_work_impact_total(total as i32);
    window.set_work_impact_shown(inferred.len() as i32);
    window.set_work_impacts_capped(total > inferred.len() as i64);
    window.set_work_impacts(ModelRc::new(VecModel::from(
        inferred
            .into_iter()
            .map(|item| WorkImpactChoice {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                label: item.label.into(),
                category: item.category.into(),
                reason: item.reason.into(),
            })
            .collect::<Vec<_>>(),
    )));
    Ok(())
}

fn set_checklist_model(
    window: &AppWindow,
    repository: &Repository,
    work_id: i64,
) -> Result<(), StorageError> {
    window.set_checklist_items(ModelRc::new(VecModel::from(
        repository
            .checklist_items(work_id)?
            .into_iter()
            .map(|item| ChecklistRow {
                id: i32::try_from(item.id).unwrap_or(i32::MAX),
                title: item.title.into(),
                completed: item.completed,
            })
            .collect::<Vec<_>>(),
    )));
    Ok(())
}

fn set_impact_summary(window: &AppWindow, impacts: &[domain::WorkImpactChoice]) {
    window.set_work_impact_total(impacts.len() as i32);
    window.set_work_impact_shown(impacts.len() as i32);
    window.set_work_impact_domains(
        impacts
            .iter()
            .filter(|item| item.category == "Domain")
            .count() as i32,
    );
    window.set_work_impact_certificates(
        impacts
            .iter()
            .filter(|item| item.category == "Certificate")
            .count() as i32,
    );
    window.set_work_impact_websites(
        impacts
            .iter()
            .filter(|item| item.category == "Website")
            .count() as i32,
    );
    window.set_work_impact_services(
        impacts
            .iter()
            .filter(|item| item.category == "Service")
            .count() as i32,
    );
    window.set_work_impact_servers(
        impacts
            .iter()
            .filter(|item| item.category == "Server")
            .count() as i32,
    );
}

fn matches_filters(server: &ServerRecord, state: &TableState) -> bool {
    let query = state.query.trim().to_lowercase();
    let selected_tag = state.tag.trim();
    let matches_tag = selected_tag.is_empty()
        || server
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(selected_tag));
    let matches_query = query.is_empty()
        || server.name.to_lowercase().contains(&query)
        || server
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&query))
        || server.ip_address.to_lowercase().contains(&query)
        || server.ports.to_lowercase().contains(&query)
        || server.environment.to_lowercase().contains(&query)
        || server.status.to_lowercase().contains(&query)
        || server.purpose.to_lowercase().contains(&query)
        || server
            .custom_values
            .iter()
            .any(|value| value.to_lowercase().contains(&query));
    matches_tag && matches_query
}

fn sort_records(records: &mut [ServerRecord], mode: usize) {
    records.sort_by(|left, right| {
        let (ordering, ascending) = match mode {
            0 => (text_compare(&left.name, &right.name), true),
            1 => (text_compare(&left.name, &right.name), false),
            2 => (ip_compare(&left.ip_address, &right.ip_address), true),
            3 => (ip_compare(&left.ip_address, &right.ip_address), false),
            4 => (first_port(&left.ports).cmp(&first_port(&right.ports)), true),
            5 => (
                first_port(&left.ports).cmp(&first_port(&right.ports)),
                false,
            ),
            6 => (text_compare(&left.environment, &right.environment), true),
            7 => (text_compare(&left.environment, &right.environment), false),
            8 => (text_compare(&left.status, &right.status), true),
            9 => (text_compare(&left.status, &right.status), false),
            10 => (text_compare(&left.purpose, &right.purpose), true),
            11 => (text_compare(&left.purpose, &right.purpose), false),
            12 => (
                text_compare(&left.tags.join(", "), &right.tags.join(", ")),
                true,
            ),
            13 => (
                text_compare(&left.tags.join(", "), &right.tags.join(", ")),
                false,
            ),
            custom => {
                let column_index = (custom - 14) / 2;
                let ascending = (custom - 14).is_multiple_of(2);
                let left_value = left
                    .custom_values
                    .get(column_index)
                    .map_or("", String::as_str);
                let right_value = right
                    .custom_values
                    .get(column_index)
                    .map_or("", String::as_str);
                (numeric_text_compare(left_value, right_value), ascending)
            }
        };
        let ordering = if ascending {
            ordering
        } else {
            ordering.reverse()
        };
        ordering.then_with(|| left.id.cmp(&right.id))
    });
}

fn text_compare(left: &str, right: &str) -> Ordering {
    left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase())
}

fn numeric_text_compare(left: &str, right: &str) -> Ordering {
    let number = |value: &str| value.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    // Keep numbers and text in separate groups so mixed columns have a total,
    // transitive ordering, rather than switching comparators for each pair.
    match (number(left), number(right)) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => text_compare(left, right),
    }
}

fn ip_compare(left: &str, right: &str) -> Ordering {
    match (left.parse::<IpAddr>(), right.parse::<IpAddr>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => text_compare(left, right),
    }
}

fn first_port(value: &str) -> u16 {
    value
        .split(',')
        .next()
        .and_then(|port| port.trim().parse().ok())
        .unwrap_or(u16::MAX)
}

fn refresh(
    window: &AppWindow,
    repository: &Repository,
    state: &TableState,
) -> Result<(), StorageError> {
    let columns = repository.server_columns()?;
    let hidden_records = repository.hidden_server_records()?;
    let active_records = repository.server_records()?;
    window.set_has_server_records(!active_records.is_empty() || !hidden_records.is_empty());
    let mut records = if state.show_hidden {
        hidden_records.clone()
    } else {
        active_records
    };
    let mut tags = records
        .iter()
        .flat_map(|server| server.tags.iter().cloned())
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_lowercase());
    tags.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    sort_records(&mut records, state.sort_mode);
    window.set_sort_mode_index(state.sort_mode as i32);
    let rows = records
        .iter()
        .filter(|server| matches_filters(server, state))
        .map(to_row)
        .collect::<Vec<_>>();

    // Clear rows before changing the custom-column model. Importing a file can
    // add several columns at once; keeping the old rows alive while the
    // headers/count are changing briefly leaves Slint with mismatched models.
    // That transient mismatch can index a custom header that does not exist
    // yet and terminate the desktop process. Replacing the models in this
    // order keeps every rendered row/header pair consistent.
    window.set_servers(ModelRc::new(VecModel::default()));
    window.set_server_count(rows.len() as i32);
    window.set_hidden_count(hidden_records.len() as i32);
    window.set_available_tags(ModelRc::new(VecModel::from(
        tags.into_iter().map(SharedString::from).collect::<Vec<_>>(),
    )));
    window.set_custom_headers(ModelRc::new(VecModel::from(
        columns
            .iter()
            .map(|column| SharedString::from(column.name.as_str()))
            .collect::<Vec<_>>(),
    )));
    window.set_custom_column_count(columns.len() as i32);
    ensure_column_widths(window, columns.len());
    window.set_servers(ModelRc::new(VecModel::from(rows)));
    Ok(())
}

fn refresh_sites(
    window: &AppWindow,
    repository: &Repository,
    state: &SiteTableState,
) -> Result<(), StorageError> {
    let hidden = repository.hidden_site_records()?;
    let mut records = if state.show_hidden {
        hidden.clone()
    } else {
        repository.site_records()?
    };
    let field = state.sort_mode / 2;
    records.sort_by(|left, right| {
        let (left_value, right_value) = match field {
            0 => (left.name.as_str(), right.name.as_str()),
            1 => (left.url.as_str(), right.url.as_str()),
            2 => (left.environment.as_str(), right.environment.as_str()),
            3 => (left.status.as_str(), right.status.as_str()),
            4 => (left.purpose.as_str(), right.purpose.as_str()),
            _ => (
                left.tags.first().map_or("", String::as_str),
                right.tags.first().map_or("", String::as_str),
            ),
        };
        let ordering = text_compare(left_value, right_value).then_with(|| left.id.cmp(&right.id));
        if state.sort_mode % 2 == 1 {
            ordering.reverse()
        } else {
            ordering
        }
    });
    let query = state.query.trim().to_lowercase();
    let rows = records
        .iter()
        .filter(|site| {
            query.is_empty()
                || [
                    site.name.as_str(),
                    site.url.as_str(),
                    site.environment.as_str(),
                    site.status.as_str(),
                    site.purpose.as_str(),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
                || site
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query))
        })
        .map(site_to_row)
        .collect::<Vec<_>>();
    window.set_sites(ModelRc::new(VecModel::from(rows.clone())));
    window.set_site_count(rows.len() as i32);
    window.set_hidden_site_count(hidden.len() as i32);
    window.set_site_sort_mode_index(state.sort_mode as i32);
    Ok(())
}

fn refresh_settings_models(
    window: &AppWindow,
    repository: &Repository,
) -> Result<(), StorageError> {
    let environments = repository.resource_options("environment", false)?;
    let statuses = repository.resource_options("status", false)?;
    let environment_default = environments
        .iter()
        .find(|option| option.is_default)
        .or_else(|| environments.first())
        .map(|option| option.label.clone())
        .unwrap_or_else(|| "Production".into());
    let status_default = statuses
        .iter()
        .find(|option| option.is_default)
        .or_else(|| statuses.first())
        .map(|option| option.label.clone())
        .unwrap_or_else(|| "Active".into());
    window.set_environment_options(ModelRc::new(VecModel::from(
        environments
            .into_iter()
            .map(|option| SharedString::from(option.label))
            .collect::<Vec<_>>(),
    )));
    window.set_status_options(ModelRc::new(VecModel::from(
        statuses
            .into_iter()
            .map(|option| SharedString::from(option.label))
            .collect::<Vec<_>>(),
    )));
    window.set_tag_suggestions(ModelRc::new(VecModel::from(
        repository
            .tag_suggestions()?
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    )));
    window.set_custom_field_labels(ModelRc::new(VecModel::from(
        repository
            .server_columns()?
            .into_iter()
            .map(|column| SharedString::from(column.name))
            .collect::<Vec<_>>(),
    )));
    if window.get_editing_id() == 0 {
        window.set_editing_environment(environment_default.clone().into());
        window.set_editing_status(status_default.clone().into());
    }
    if window.get_editing_site_id() == 0 {
        window.set_editing_site_environment(environment_default.into());
        window.set_editing_site_status(status_default.into());
    }
    Ok(())
}

fn apply_preferences(window: &AppWindow, preferences: &WorkspacePreferences) {
    window.set_theme_index(preferences.theme_index as i32);
    window.set_font_size_step(preferences.font_size_step as i32);
    window.set_table_density(preferences.table_density as i32);
    window.set_show_cell_tooltips(preferences.show_cell_tooltips);
    window.set_fit_columns_on_open(preferences.fit_columns_on_open);
    window.set_import_duplicate_policy(preferences.import_duplicate_policy.index() as i32);
    window.set_backup_retention(preferences.backup_retention as i32);
}

fn site_mapping_options() -> Vec<SharedString> {
    [
        "Skip",
        "Name",
        "URL",
        "Environment",
        "Status",
        "Purpose",
        "Tags",
    ]
    .into_iter()
    .map(SharedString::from)
    .collect()
}

fn automatic_site_mappings(data: &TabularData) -> Vec<usize> {
    let mut mappings = data
        .headers
        .iter()
        .map(|header| match normalize_header(header).as_str() {
            "name" | "sitename" | "website" | "title" => 1,
            "url" | "websiteurl" | "siteurl" | "address" => 2,
            "environment" | "env" | "stage" => 3,
            "status" | "state" => 4,
            "purpose" | "description" | "desc" | "notes" => 5,
            "tag" | "tags" | "labels" => 6,
            _ => 0,
        })
        .collect::<Vec<_>>();
    for required in [1, 2] {
        let mut seen = false;
        for mapping in &mut mappings {
            if *mapping == required {
                if seen {
                    *mapping = 0;
                } else {
                    seen = true;
                }
            }
        }
    }
    mappings
}

fn site_import_column_rows(data: &TabularData, mappings: &[usize]) -> Vec<ImportColumnRow> {
    import_column_rows(data, mappings)
}

fn site_import_targets(mappings: &[usize]) -> Option<Vec<SiteImportTarget>> {
    mappings
        .iter()
        .map(|mapping| match mapping {
            0 => Some(SiteImportTarget::Skip),
            1 => Some(SiteImportTarget::Name),
            2 => Some(SiteImportTarget::Url),
            3 => Some(SiteImportTarget::Environment),
            4 => Some(SiteImportTarget::Status),
            5 => Some(SiteImportTarget::Purpose),
            6 => Some(SiteImportTarget::Tags),
            _ => None,
        })
        .collect()
}

fn ensure_column_widths(window: &AppWindow, custom_count: usize) {
    let mut widths = window.get_column_widths().iter().collect::<Vec<_>>();
    let standard_defaults = [160.0, 140.0, 110.0, 130.0, 110.0, 220.0, 170.0];
    while widths.len() < standard_defaults.len() {
        widths.push(standard_defaults[widths.len()]);
    }
    widths.resize(7 + custom_count, 160.0);
    for width in &mut widths {
        *width = width.clamp(96.0, 420.0);
    }
    widths.truncate(7 + custom_count);
    window.set_column_widths(ModelRc::new(VecModel::from(widths.clone())));
    update_table_geometry(window, &widths);
}

fn table_data_width(widths: &[f32]) -> f32 {
    widths.iter().sum()
}

fn table_width_for(widths: &[f32]) -> f32 {
    (table_data_width(widths) + 116.0).max(816.0)
}

fn update_table_geometry(window: &AppWindow, widths: &[f32]) {
    window.set_table_data_width(table_data_width(widths));
    window.set_table_width(table_width_for(widths));
}

fn update_column_width(window: &AppWindow, index: usize, width: f32) {
    let model = window.get_column_widths();
    let mut widths = model.iter().collect::<Vec<_>>();
    if let Some(current) = widths.get_mut(index) {
        *current = width.clamp(96.0, 420.0);
        window.set_column_widths(ModelRc::new(VecModel::from(widths.clone())));
        update_table_geometry(window, &widths);
    }
}

fn auto_fit_column_widths(window: &AppWindow) -> Vec<f32> {
    let headers = window
        .get_custom_headers()
        .iter()
        .map(|header| header.to_string())
        .collect::<Vec<_>>();
    let rows = (0..window.get_servers().row_count())
        .filter_map(|row_index| window.get_servers().row_data(row_index))
        .map(|row| {
            let mut values = vec![
                row.name.to_string(),
                row.ip_address.to_string(),
                row.ports.to_string(),
                row.environment.to_string(),
                row.status.to_string(),
                row.purpose.to_string(),
                row.tags.to_string(),
            ];
            values.extend(row.custom_values.iter().map(|value| value.to_string()));
            values
        })
        .collect::<Vec<_>>();
    fitted_column_widths(&headers, &rows)
}

fn fitted_column_widths(custom_headers: &[String], rows: &[Vec<String>]) -> Vec<f32> {
    let mut widths = custom_headers
        .iter()
        .map(|header| (header.chars().count() as f32 * 8.0 + 36.0).clamp(96.0, 420.0))
        .collect::<Vec<_>>();
    for (index, width) in [160.0, 140.0, 110.0, 130.0, 110.0, 220.0, 170.0]
        .into_iter()
        .enumerate()
    {
        widths.insert(index, width);
    }
    for (index, width) in widths.iter_mut().enumerate() {
        for row in rows {
            let value = row.get(index).map_or("", String::as_str);
            *width = (*width).max(value.chars().count() as f32 * 8.0 + 36.0);
        }
        *width = (*width).clamp(96.0, 420.0);
    }
    widths
}

fn mapping_options(columns: &[ServerColumn]) -> Vec<SharedString> {
    let mut options = vec![
        "Skip".into(),
        "Name".into(),
        "IP Address".into(),
        "Ports".into(),
        "Environment".into(),
        "Status".into(),
        "Purpose".into(),
        "Tags".into(),
    ];
    options.extend(
        columns
            .iter()
            .map(|column| SharedString::from(format!("Existing: {}", column.name))),
    );
    options.extend(
        SUGGESTED_CUSTOM_FIELDS
            .iter()
            .map(|field| SharedString::from(format!("Suggested field: {field}"))),
    );
    options.push("New custom column".into());
    options
}

fn default_mapping(header: &str, columns: &[ServerColumn]) -> usize {
    let normalized = header
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "tag" | "tags" | "label" | "labels" | "category" | "categories"
    ) {
        return 7;
    }
    if matches!(
        normalized.as_str(),
        "ip" | "ipaddress" | "address" | "serverip" | "publicip" | "privateip"
    ) {
        return 2;
    }
    if matches!(
        normalized.as_str(),
        "port" | "ports" | "openports" | "serviceport" | "serviceports"
    ) {
        return 3;
    }
    if matches!(
        normalized.as_str(),
        "name" | "servername" | "hostname" | "title"
    ) {
        return 1;
    }
    if matches!(normalized.as_str(), "environment" | "env" | "stage") {
        return 4;
    }
    if matches!(normalized.as_str(), "status" | "state") {
        return 5;
    }
    if matches!(normalized.as_str(), "purpose" | "description" | "desc") {
        return 6;
    }
    if let Some(index) = columns
        .iter()
        .position(|column| column.name.eq_ignore_ascii_case(header.trim()))
    {
        return 8 + index;
    }
    if let Some(index) = SUGGESTED_CUSTOM_FIELDS
        .iter()
        .position(|field| normalized == normalize_header(field))
    {
        return 8 + columns.len() + index;
    }
    8 + columns.len() + SUGGESTED_CUSTOM_FIELDS.len()
}

fn automatic_mappings(data: &TabularData, columns: &[ServerColumn]) -> Vec<usize> {
    let mut mappings = data
        .headers
        .iter()
        .map(|header| default_mapping(header, columns))
        .collect::<Vec<_>>();
    let mut has_ip = mappings.contains(&2);
    let mut has_ports = mappings.contains(&3);
    for (index, values) in column_values(data).into_iter().enumerate() {
        if !has_ip && values.iter().any(|value| value.parse::<IpAddr>().is_ok()) {
            mappings[index] = 2;
            has_ip = true;
            continue;
        }
        if !has_ports
            && values.iter().any(|value| {
                !value.is_empty()
                    && value
                        .split(',')
                        .all(|part| part.trim().parse::<u16>().is_ok_and(|port| port > 0))
            })
        {
            mappings[index] = 3;
            has_ports = true;
        }
    }
    dedupe_required_mappings(&mut mappings);
    mappings
}

fn column_values(data: &TabularData) -> Vec<Vec<String>> {
    (0..data.headers.len())
        .map(|index| {
            data.rows
                .iter()
                .take(10)
                .filter_map(|row| row.get(index))
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn dedupe_required_mappings(mappings: &mut [usize]) {
    for required in 1..=7 {
        let mut seen = false;
        for mapping in mappings.iter_mut() {
            if *mapping == required {
                if seen {
                    *mapping = 0;
                } else {
                    seen = true;
                }
            }
        }
    }
}

fn normalize_header(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn import_target(
    source_index: usize,
    mapping_index: usize,
    session: &ImportSession,
) -> Option<ServerImportTarget> {
    let suggested_start = 8 + session.existing_columns.len();
    let new_custom_index = suggested_start + SUGGESTED_CUSTOM_FIELDS.len();
    match mapping_index {
        0 => Some(ServerImportTarget::Skip),
        1 => Some(ServerImportTarget::Name),
        2 => Some(ServerImportTarget::IpAddress),
        3 => Some(ServerImportTarget::Ports),
        4 => Some(ServerImportTarget::Environment),
        5 => Some(ServerImportTarget::Status),
        6 => Some(ServerImportTarget::Purpose),
        7 => Some(ServerImportTarget::Tags),
        index if index < 8 + session.existing_columns.len() => session
            .existing_columns
            .get(index - 8)
            .map(|column| ServerImportTarget::ExistingCustom(column.id)),
        index if index < new_custom_index => SUGGESTED_CUSTOM_FIELDS
            .get(index - suggested_start)
            .map(|field| ServerImportTarget::NewCustom((*field).to_owned())),
        index if index == new_custom_index => session
            .data
            .as_ref()
            .and_then(|data| data.headers.get(source_index))
            .map(|header| ServerImportTarget::NewCustom(header.clone())),
        _ => None,
    }
}

fn import_column_rows(data: &TabularData, mappings: &[usize]) -> Vec<ImportColumnRow> {
    data.headers
        .iter()
        .enumerate()
        .map(|(index, header)| ImportColumnRow {
            index: index as i32,
            source_name: header.as_str().into(),
            sample: data
                .rows
                .iter()
                .take(3)
                .filter_map(|row| row.get(index))
                .filter(|value| !value.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
                .into(),
            mapping_index: mappings[index] as i32,
        })
        .collect()
}

fn show_error(window: &AppWindow, error: &dyn std::fmt::Display) {
    window.set_status_message(SharedString::from(error.to_string()));
    window.set_status_is_error(true);
}

fn show_status(window: &AppWindow, message: &str) {
    window.set_status_message(message.into());
    window.set_status_is_error(false);
}

fn clear_editor(window: &AppWindow) {
    window.set_editing_id(0);
    window.set_editing_name(SharedString::new());
    window.set_editing_tags(SharedString::new());
    window.set_editing_ip_address(SharedString::new());
    window.set_editing_ports(SharedString::new());
    window.set_editing_environment("production".into());
    window.set_editing_status("active".into());
    window.set_editing_purpose(SharedString::new());
}

#[cfg(debug_assertions)]
fn apply_desktop_audit_fixture(window: &AppWindow) {
    if let Ok(size) = std::env::var("ITGLA_AUDIT_WINDOW_SIZE")
        && let Some((width, height)) = size.split_once('x').and_then(|(width, height)| {
            Some((width.parse::<u32>().ok()?, height.parse::<u32>().ok()?))
        })
    {
        window.window().set_size(PhysicalSize::new(width, height));
    }

    match std::env::var("ITGLA_AUDIT_VIEW").as_deref() {
        Ok("servers") => {
            window.set_active_tab(0);
            window.set_onboarding_open(false);
        }
        Ok("sites") | Ok("projects") => {
            window.set_active_tab(1);
            window.set_onboarding_open(false);
        }
        Ok("settings") => {
            window.set_active_tab(2);
            window.set_onboarding_open(false);
        }
        Ok("import") => {
            window.set_active_tab(0);
            window.set_onboarding_open(false);
            window.set_import_dialog_open(true);
        }
        Ok(view @ ("inventory" | "domains")) => {
            window.set_active_tab(4);
            window.set_onboarding_open(false);
            window.invoke_inventory_new(if view == "domains" { 1 } else { 0 });
            if let Some(row) = window.get_inventory_rows().row_data(0) {
                window.invoke_inventory_select(row.id);
            }
        }
        Ok("guide") => window.set_onboarding_open(true),
        _ => {}
    }

    if std::env::var_os("ITGLA_AUDIT_SERVER_FIELDS").is_some() {
        window.set_editing_name("audit-server".into());
        window.set_editing_tags("audit".into());
        window.set_editing_ip_address("203.0.113.10".into());
        window.set_editing_ports("22, 443".into());
        window.set_editing_environment("test".into());
        window.set_editing_status("active".into());
        window.set_editing_purpose("Interaction audit".into());
    }
}

#[cfg(not(debug_assertions))]
fn apply_desktop_audit_fixture(_window: &AppWindow) {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Rc::new(RefCell::new(Repository::open_default()?));
    let real_repository = Rc::new(RefCell::new(None::<Repository>));
    let table_state = Rc::new(RefCell::new(TableState::default()));
    let site_table_state = Rc::new(RefCell::new(SiteTableState::default()));
    let import_session = Rc::new(RefCell::new(ImportSession::default()));
    let site_import_session = Rc::new(RefCell::new(SiteImportSession::default()));
    let work_list_state = Rc::new(RefCell::new(WorkListState::default()));
    let window = AppWindow::new()?;
    operational::setup(&window, repository.clone());
    let preferences = repository.borrow().workspace_preferences();
    apply_preferences(&window, &preferences);
    window.set_column_widths(ModelRc::new(VecModel::from(
        repository.borrow().column_widths()?,
    )));
    let project_rows = repository
        .borrow()
        .project_plans()?
        .into_iter()
        .map(|project| ProjectRow {
            name: project.name.into(),
            description: project.description.into(),
            start_at: project.start_at.into(),
            priority: project.priority.into(),
            launch_at: project.launch_at.into(),
        })
        .collect::<Vec<_>>();
    let operation_rows = repository
        .borrow()
        .operations()?
        .into_iter()
        .map(|operation| OperationRow {
            name: operation.name.into(),
            publish_at: operation.publish_at.into(),
            media: operation.media.into(),
            channel: operation.channel.into(),
        })
        .collect::<Vec<_>>();
    let priorities = Rc::new(VecModel::from(
        repository
            .borrow()
            .priority_options()?
            .into_iter()
            .map(|option| SharedString::from(option.label))
            .collect::<Vec<_>>(),
    ));
    let media_options = Rc::new(VecModel::from(
        repository
            .borrow()
            .media_options()?
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ));
    window.set_projects(ModelRc::new(VecModel::from(project_rows)));
    window.set_operations(ModelRc::new(VecModel::from(operation_rows)));
    set_work_rows(&window, &repository.borrow(), &work_list_state.borrow())?;
    window.set_work_projects(ModelRc::new(VecModel::default()));
    window.set_work_servers(ModelRc::new(VecModel::default()));
    window.set_work_assets(ModelRc::new(VecModel::default()));
    window.set_work_impacts(ModelRc::new(VecModel::default()));
    window.set_checklist_items(ModelRc::new(VecModel::default()));
    window.set_work_impacts_capped(false);
    window.set_work_impact_ignored(0);
    set_impact_summary(&window, &[]);
    window.set_work_health_message("Add a project or resource scope".into());
    window.set_work_health_has_warnings(true);
    window.set_work_health_action(1);
    window.set_work_health_action_label("Review scope".into());
    window.set_priority_options(ModelRc::from(priorities.clone()));
    window.set_media_options(ModelRc::from(media_options.clone()));
    refresh(&window, &repository.borrow(), &table_state.borrow())?;
    refresh_sites(&window, &repository.borrow(), &site_table_state.borrow())?;
    refresh_settings_models(&window, &repository.borrow())?;
    if preferences.fit_columns_on_open {
        let widths = auto_fit_column_widths(&window);
        window.set_column_widths(ModelRc::new(VecModel::from(widths.clone())));
        update_table_geometry(&window, &widths);
        window.invoke_auto_fit_site_columns();
    }
    window.set_onboarding_open(!window.get_has_server_records());
    apply_desktop_audit_fixture(&window);

    let weak = window.as_weak();
    window.on_tab_selected(move |tab| {
        if let Some(window) = weak.upgrade() {
            operational::clear_credentials(&window);
            window.set_active_tab(tab);
            if tab == 4 {
                window.invoke_inventory_new(window.get_inventory_form().kind);
            }
            window.set_status_message(SharedString::new());
        }
    });

    let weak = window.as_weak();
    window.on_open_server_inventory(move |id| {
        if let Some(window) = weak.upgrade() {
            operational::clear_credentials(&window);
            window.set_inventory_server_filter(id);
            window.set_active_tab(4);
            window.invoke_inventory_new(0);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_work_list_filter_changed(move |query, kind, status, sort_mode| {
        let Some(window) = weak.upgrade() else { return };
        {
            let mut state = callback_work_list_state.borrow_mut();
            state.query = query.to_string();
            state.kind_filter = kind;
            state.status_filter = status;
            state.sort_mode = sort_mode;
        }
        if let Err(error) = set_work_rows(
            &window,
            &callback_repository.borrow(),
            &callback_work_list_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_work_quick_status(move |id, status| {
        let Some(window) = weak.upgrade() else { return };
        let Some(status) = WorkStatus::parse(status.as_str()) else {
            show_error(&window, &"Unknown Work status");
            return;
        };
        match callback_repository
            .borrow()
            .set_work_status(i64::from(id), status)
        {
            Ok(()) => {
                let _ = set_work_rows(
                    &window,
                    &callback_repository.borrow(),
                    &callback_work_list_state.borrow(),
                );
                if window.get_selected_work_id() == id
                    && let Ok(work_items) = callback_repository.borrow().work_items()
                    && let Some(work) = work_items.into_iter().find(|item| item.id == i64::from(id))
                {
                    set_work_editor(&window, &work);
                    let _ = set_work_scope_models(
                        &window,
                        &callback_repository.borrow(),
                        i64::from(id),
                    );
                }
                show_status(&window, &format!("Work marked {}", status.label()));
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_checklist_toggled(move |id, completed| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository
            .borrow()
            .set_checklist_item_completed(i64::from(id), completed)
        {
            Ok(()) => {
                let work_id = i64::from(window.get_selected_work_id());
                let _ = set_checklist_model(&window, &callback_repository.borrow(), work_id);
                let _ = set_work_rows(
                    &window,
                    &callback_repository.borrow(),
                    &callback_work_list_state.borrow(),
                );
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_checklist_added(move |work_id, title| {
        let Some(window) = weak.upgrade() else { return };
        if work_id <= 0 {
            return;
        }
        match callback_repository
            .borrow()
            .add_checklist_item(i64::from(work_id), title.as_str())
        {
            Ok(_) => {
                window.set_checklist_input(SharedString::new());
                let _ =
                    set_checklist_model(&window, &callback_repository.borrow(), i64::from(work_id));
                let _ = set_work_rows(
                    &window,
                    &callback_repository.borrow(),
                    &callback_work_list_state.borrow(),
                );
                show_status(&window, "Checklist item added");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_checklist_deleted(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository
            .borrow()
            .delete_checklist_item(i64::from(id))
        {
            Ok(()) => {
                let work_id = i64::from(window.get_selected_work_id());
                let _ = set_checklist_model(&window, &callback_repository.borrow(), work_id);
                let _ = set_work_rows(
                    &window,
                    &callback_repository.borrow(),
                    &callback_work_list_state.borrow(),
                );
                show_status(&window, "Checklist item deleted");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_checklist_moved(move |id, direction| {
        let Some(window) = weak.upgrade() else { return };
        let move_result = callback_repository
            .borrow_mut()
            .move_checklist_item(i64::from(id), i64::from(direction));
        match move_result {
            Ok(()) => {
                let work_id = i64::from(window.get_selected_work_id());
                let _ = set_checklist_model(&window, &callback_repository.borrow(), work_id);
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    window.on_new_work(move || {
        if let Some(window) = weak.upgrade() {
            clear_work_editor(&window);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_work_selected(move |id| {
        let Some(window) = weak.upgrade() else { return };
        let Ok(work_items) = callback_repository.borrow().work_items() else {
            return;
        };
        if let Some(work) = work_items.into_iter().find(|work| work.id == i64::from(id)) {
            set_work_editor(&window, &work);
            let _ = set_work_scope_models(&window, &callback_repository.borrow(), i64::from(id));
            let _ = set_checklist_model(&window, &callback_repository.borrow(), i64::from(id));
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_save_work(
        move |id, title, kind, status, priority, owner, start_at, due_at, notes| {
            let Some(window) = weak.upgrade() else { return };
        let Some(kind) = WorkKind::parse(kind.trim()) else {
            show_error(&window, &"Type must be change, task, renewal, incident, migration, launch, or maintenance");
            return;
        };
        let Some(status) = WorkStatus::parse(status.trim()) else {
            show_error(&window, &"Status must be inbox, planned, ready, in_progress, blocked, done, or cancelled");
            return;
        };
            if status == WorkStatus::Ready && id > 0 {
                match callback_repository
                    .borrow()
                    .inferred_work_assets(i64::from(id))
                {
                    Ok(impacts) if !impacts.is_empty() => {
                        show_error(
                            &window,
                            &"This work still has potential impact. Review it and use Add or Add all before marking Ready.",
                        );
                        return;
                    }
                    Err(error) => {
                        show_error(&window, &error);
                        return;
                    }
                    _ => {}
                }
            }
            let draft = WorkDraft {
                title: title.to_string(),
                kind,
                status,
                priority: priority.to_string(),
                owner: owner.to_string(),
                start_at: start_at.to_string(),
                due_at: due_at.to_string(),
                notes: notes.to_string(),
            };
            let save_result = callback_repository
                .borrow_mut()
                .save_work_item((id > 0).then_some(i64::from(id)), &draft);
            match save_result {
            Ok(saved_id) => {
                let project_ids = window
                    .get_work_projects()
                    .iter()
                    .filter(|item| item.selected)
                    .map(|item| i64::from(item.id))
                    .collect::<Vec<_>>();
                let server_ids = window
                    .get_work_servers()
                    .iter()
                    .filter(|item| item.selected)
                    .map(|item| i64::from(item.id))
                    .collect::<Vec<_>>();
                let asset_ids = window
                    .get_work_assets()
                    .iter()
                    .filter(|item| item.selected)
                    .map(|item| i64::from(item.id))
                    .collect::<Vec<_>>();
                if let Err(error) = callback_repository
                    .borrow_mut()
                    .set_work_scope(saved_id, &project_ids, &server_ids)
                {
                    show_error(&window, &error);
                    return;
                }
                if let Err(error) = callback_repository
                    .borrow_mut()
                    .set_work_asset_scope(saved_id, &asset_ids)
                {
                    show_error(&window, &error);
                    return;
                }
                if let Ok(work_items) = callback_repository.borrow().work_items() {
                    let _ = set_work_rows(
                        &window,
                        &callback_repository.borrow(),
                        &callback_work_list_state.borrow(),
                    );
                    if let Some(work) = work_items.into_iter().find(|work| work.id == saved_id)
                    {
                        set_work_editor(&window, &work);
                        let _ = set_work_scope_models(&window, &callback_repository.borrow(), saved_id);
                        let _ = set_checklist_model(&window, &callback_repository.borrow(), saved_id);
                    }
                    }
                    show_status(&window, "Work saved");
                }
                Err(error) => show_error(&window, &error),
            }
        },
    );

    let callback_repository = Rc::clone(&repository);
    let callback_work_list_state = Rc::clone(&work_list_state);
    let weak = window.as_weak();
    window.on_archive_work(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository
            .borrow()
            .archive_work_item(i64::from(id))
        {
            Ok(()) => {
                let _ = set_work_rows(
                    &window,
                    &callback_repository.borrow(),
                    &callback_work_list_state.borrow(),
                );
                clear_work_editor(&window);
                window.set_work_projects(ModelRc::new(VecModel::default()));
                window.set_work_servers(ModelRc::new(VecModel::default()));
                window.set_work_assets(ModelRc::new(VecModel::default()));
                window.set_work_impacts(ModelRc::new(VecModel::default()));
                window.set_work_impacts_capped(false);
                set_impact_summary(&window, &[]);
                show_status(&window, "Work archived");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    window.on_work_project_toggled(move |id, selected| {
        let Some(window) = weak.upgrade() else { return };
        let model = window.get_work_projects();
        for index in 0..model.row_count() {
            if let Some(mut item) = model.row_data(index)
                && item.id == id
            {
                item.selected = selected;
                model.set_row_data(index, item);
                window.set_work_project_count(
                    model.iter().filter(|item| item.selected).count() as i32
                );
                break;
            }
        }
    });

    let weak = window.as_weak();
    window.on_work_server_toggled(move |id, selected| {
        let Some(window) = weak.upgrade() else { return };
        let model = window.get_work_servers();
        for index in 0..model.row_count() {
            if let Some(mut item) = model.row_data(index)
                && item.id == id
            {
                item.selected = selected;
                model.set_row_data(index, item);
                window.set_work_server_count(
                    model.iter().filter(|item| item.selected).count() as i32,
                );
                break;
            }
        }
    });

    let weak = window.as_weak();
    window.on_work_asset_toggled(move |id, selected| {
        let Some(window) = weak.upgrade() else { return };
        let model = window.get_work_assets();
        for index in 0..model.row_count() {
            if let Some(mut item) = model.row_data(index)
                && item.id == id
            {
                item.selected = selected;
                if selected && item.origin.is_empty() {
                    item.origin = "direct".into();
                }
                model.set_row_data(index, item);
                window
                    .set_work_asset_count(model.iter().filter(|item| item.selected).count() as i32);
                break;
            }
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_impact_promoted(move |id| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id <= 0 {
            return;
        }
        let promote_result = callback_repository
            .borrow_mut()
            .add_work_asset_to_scope(work_id, i64::from(id));
        match promote_result {
            Ok(()) => {
                let _ = set_work_scope_models(&window, &callback_repository.borrow(), work_id);
                show_status(&window, "Impact added to scope");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_impact_excluded(move |id| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id <= 0 {
            return;
        }
        match callback_repository
            .borrow()
            .exclude_work_impact(work_id, i64::from(id))
        {
            Ok(()) => {
                let _ = set_work_scope_models(&window, &callback_repository.borrow(), work_id);
                show_status(&window, "Impact reviewed and ignored");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_impact_exclusions_reset(move || {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id <= 0 {
            return;
        }
        match callback_repository
            .borrow()
            .reset_excluded_work_impacts(work_id)
        {
            Ok(()) => {
                let _ = set_work_scope_models(&window, &callback_repository.borrow(), work_id);
                show_status(&window, "Ignored impacts restored for review");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_all_impacts_promoted(move |depth, filter_index| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id <= 0 {
            return;
        }
        let category = match filter_index {
            1 => Some("Domain"),
            2 => Some("Certificate"),
            3 => Some("Website"),
            4 => Some("Service"),
            5 => Some("Server"),
            _ => None,
        };
        let promote_result = callback_repository
            .borrow_mut()
            .add_inferred_work_assets_with_depth_and_category(work_id, i64::from(depth), category);
        match promote_result {
            Ok(added) => {
                let _ = set_work_scope_models(&window, &callback_repository.borrow(), work_id);
                show_status(&window, &format!("Added {added} impact(s) to scope"));
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_impact_depth_changed(move |depth| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id > 0 {
            let _ = callback_repository.borrow().set_work_impact_view(
                work_id,
                i64::from(depth),
                i64::from(window.get_work_impact_filter_index()),
            );
            let _ = set_work_impact_depth(&window, &callback_repository.borrow(), work_id, depth);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_impact_filter_changed(move |filter| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id > 0 {
            let _ = callback_repository.borrow().set_work_impact_view(
                work_id,
                i64::from(window.get_work_impact_depth()),
                i64::from(filter),
            );
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_work_health_action_requested(move |action| {
        let Some(window) = weak.upgrade() else { return };
        let work_id = i64::from(window.get_selected_work_id());
        if work_id <= 0 {
            show_status(&window, "Select projects, servers, or resources below");
            return;
        }
        match action {
            1 => show_status(
                &window,
                "Review the project, server, and resource scope below",
            ),
            2 => {
                window.set_work_impact_depth(1);
                window.set_work_impact_filter_index(0);
                let _ = callback_repository
                    .borrow()
                    .set_work_impact_view(work_id, 1, 0);
                let _ = set_work_impact_depth(&window, &callback_repository.borrow(), work_id, 1);
                show_status(&window, "Showing all direct impact suggestions");
            }
            _ => {}
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_font_size_changed(move |step| {
        let step = step.clamp(0, 2);
        if callback_repository
            .borrow()
            .set_font_size_step(i64::from(step))
            .is_err()
        {
            return;
        }
        if let Some(window) = weak.upgrade() {
            window.set_font_size_step(step);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_preferences_changed(
        move |theme, font, density, tooltips, fit, duplicate, retention| {
            let Some(window) = weak.upgrade() else { return };
            let mut preferences = callback_repository.borrow().workspace_preferences();
            preferences.theme_index = i64::from(theme.clamp(0, 3));
            preferences.font_size_step = i64::from(font.clamp(0, 2));
            preferences.table_density = i64::from(density.clamp(0, 2));
            preferences.show_cell_tooltips = tooltips;
            preferences.fit_columns_on_open = fit;
            preferences.import_duplicate_policy =
                ImportDuplicatePolicy::from_index(i64::from(duplicate));
            preferences.backup_retention = retention.clamp(1, 30) as usize;
            match callback_repository
                .borrow()
                .set_workspace_preferences(&preferences)
            {
                Ok(()) => {
                    apply_preferences(&window, &preferences);
                    show_status(&window, "Preferences saved");
                }
                Err(error) => show_error(&window, &error),
            }
        },
    );

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_add_managed_option(move |kind, label| {
        let Some(window) = weak.upgrade() else { return };
        if label.trim().is_empty() {
            return;
        }
        let save_result = callback_repository.borrow_mut().save_resource_option(
            None,
            kind.as_str(),
            label.as_str(),
            "neutral",
            "slate",
            false,
        );
        match save_result {
            Ok(_) => {
                if kind.as_str() == "status" {
                    window.set_settings_status_input(SharedString::new());
                } else {
                    window.set_settings_option_input(SharedString::new());
                }
                let _ = refresh_settings_models(&window, &callback_repository.borrow());
                show_status(&window, "Managed choice added");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_add_managed_tag(move |label| {
        let Some(window) = weak.upgrade() else { return };
        if label.trim().is_empty() {
            return;
        }
        let save_result =
            callback_repository
                .borrow_mut()
                .save_managed_tag(None, label.as_str(), "slate");
        match save_result {
            Ok(_) => {
                window.set_settings_tag_input(SharedString::new());
                let _ = refresh_settings_models(&window, &callback_repository.borrow());
                show_status(&window, "Tag suggestion added");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    let weak = window.as_weak();
    window.on_add_custom_field(move |name, value_type| {
        let Some(window) = weak.upgrade() else { return };
        let types = ["text", "number", "date", "url", "boolean"];
        let value_type = types
            .get(value_type.max(0) as usize)
            .copied()
            .unwrap_or("text");
        let save_result = callback_repository.borrow_mut().save_server_column(
            None,
            name.as_str(),
            value_type,
            "",
        );
        match save_result {
            Ok(_) => {
                window.set_settings_field_input(SharedString::new());
                let _ = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                );
                let _ = refresh_settings_models(&window, &callback_repository.borrow());
                show_status(&window, "Custom field added");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_create_managed_backup(move || {
        let Some(window) = weak.upgrade() else { return };
        let retention = window.get_backup_retention().clamp(1, 30) as usize;
        match callback_repository
            .borrow()
            .create_managed_backup(retention)
        {
            Ok(path) => show_status(&window, &format!("Backup created: {}", path.display())),
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_real_repository = Rc::clone(&real_repository);
    let callback_state = Rc::clone(&table_state);
    let callback_site_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_enter_demo(move || {
        let Some(window) = weak.upgrade() else { return };
        if callback_real_repository.borrow().is_some() {
            return;
        }
        let demo = Repository::in_memory().and_then(|mut repository| {
            repository.reset_demo_fixture()?;
            Ok(repository)
        });
        match demo {
            Ok(demo) => {
                let real = std::mem::replace(&mut *callback_repository.borrow_mut(), demo);
                *callback_real_repository.borrow_mut() = Some(real);
                operational::clear_credentials(&window);
                window.set_inventory_server_filter(0);
                window.set_demo_mode(true);
                let _ = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                );
                let _ = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_site_state.borrow(),
                );
                let _ = refresh_settings_models(&window, &callback_repository.borrow());
                window.set_active_tab(0);
                show_status(&window, "Demo workspace opened — real data is untouched");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    let callback_site_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_reset_demo(move || {
        let Some(window) = weak.upgrade() else { return };
        if !window.get_demo_mode() {
            return;
        }
        let reset_result = callback_repository.borrow_mut().reset_demo_fixture();
        match reset_result {
            Ok(()) => {
                let _ = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                );
                let _ = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_site_state.borrow(),
                );
                show_status(&window, "Demo reset");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_real_repository = Rc::clone(&real_repository);
    let callback_state = Rc::clone(&table_state);
    let callback_site_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_exit_demo(move || {
        let Some(window) = weak.upgrade() else { return };
        let Some(real) = callback_real_repository.borrow_mut().take() else {
            return;
        };
        let _demo = std::mem::replace(&mut *callback_repository.borrow_mut(), real);
        operational::clear_credentials(&window);
        window.set_inventory_server_filter(0);
        window.set_demo_mode(false);
        let preferences = callback_repository.borrow().workspace_preferences();
        apply_preferences(&window, &preferences);
        let _ = refresh(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        );
        let _ = refresh_sites(
            &window,
            &callback_repository.borrow(),
            &callback_site_state.borrow(),
        );
        let _ = refresh_settings_models(&window, &callback_repository.borrow());
        window.set_active_tab(2);
        show_status(&window, "Returned to your workspace");
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_column_width_changed(move |index, width| {
        let Some(window) = weak.upgrade() else { return };
        update_column_width(&window, index.max(0) as usize, width);
        let widths = window.get_column_widths().iter().collect::<Vec<_>>();
        let _ = callback_repository.borrow().set_column_widths(&widths);
    });

    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_auto_fit_columns(move || {
        if let Some(window) = weak.upgrade() {
            let widths = auto_fit_column_widths(&window);
            window.set_column_widths(ModelRc::new(VecModel::from(widths)));
            let saved = window.get_column_widths().iter().collect::<Vec<_>>();
            update_table_geometry(&window, &saved);
            window.set_server_table_scroll_x(0.0);
            let _ = callback_repository.borrow().set_column_widths(&saved);
        }
    });

    let weak = window.as_weak();
    window.on_site_column_width_changed(move |index, width| {
        let Some(window) = weak.upgrade() else { return };
        let mut widths = window.get_site_column_widths().iter().collect::<Vec<_>>();
        if let Some(current) = widths.get_mut(index.max(0) as usize) {
            *current = width.clamp(96.0, 420.0);
            window.set_site_column_widths(ModelRc::new(VecModel::from(widths.clone())));
            window.set_site_table_width((widths.iter().sum::<f32>() + 116.0).max(816.0));
        }
    });

    let weak = window.as_weak();
    window.on_auto_fit_site_columns(move || {
        let Some(window) = weak.upgrade() else { return };
        let mut widths = vec![180.0_f32, 260.0, 140.0, 120.0, 220.0, 180.0];
        for row in window.get_sites().iter() {
            let values = [
                row.name,
                row.url,
                row.environment,
                row.status,
                row.purpose,
                row.tags,
            ];
            for (index, value) in values.iter().enumerate() {
                widths[index] = widths[index]
                    .max(value.chars().count() as f32 * 8.0 + 36.0)
                    .clamp(96.0, 420.0);
            }
        }
        window.set_site_column_widths(ModelRc::new(VecModel::from(widths.clone())));
        window.set_site_table_width((widths.iter().sum::<f32>() + 116.0).max(816.0));
        window.set_site_table_scroll_x(0.0);
    });

    let weak = window.as_weak();
    window.on_edit_cell(move |id, column, value| {
        if let Some(window) = weak.upgrade() {
            window.set_editing_cell_is_site(false);
            window.set_editing_cell_id(id);
            window.set_editing_cell_column(column);
            window.set_editing_cell_value(value);
        }
    });

    let weak = window.as_weak();
    window.on_edit_site_cell(move |id, column, value| {
        if let Some(window) = weak.upgrade() {
            window.set_editing_cell_is_site(true);
            window.set_editing_cell_id(id);
            window.set_editing_cell_column(column);
            window.set_editing_cell_value(value);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_save_site_cell_edit(move |id, column, value| {
        let Some(window) = weak.upgrade() else { return };
        let update_result = callback_repository.borrow_mut().update_site_cell(
            i64::from(id),
            column.max(0) as usize,
            value.as_str(),
        );
        match update_result {
            Ok(()) => {
                window.set_editing_cell_column(-1);
                if let Err(error) = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                } else {
                    show_status(&window, "Cell saved");
                }
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    let weak = window.as_weak();
    window.on_save_cell_edit(move |id, column, value| {
        let Some(window) = weak.upgrade() else { return };
        let update_result = callback_repository.borrow_mut().update_server_cell(
            i64::from(id),
            column.max(0) as usize,
            value.as_str(),
        );
        match update_result {
            Ok(()) => {
                window.set_editing_cell_column(-1);
                if let Err(error) = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                } else {
                    show_status(&window, "Cell saved");
                }
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let priorities_for_add = Rc::clone(&priorities);
    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_add_priority(move |value| {
        let value = value.trim();
        if value.is_empty()
            || priorities_for_add
                .iter()
                .any(|existing| existing.as_str().eq_ignore_ascii_case(value))
        {
            return;
        }
        if callback_repository
            .borrow_mut()
            .add_priority_option(value, "#126a57")
            .is_err()
        {
            return;
        }
        priorities_for_add.push(value.into());
        if let Some(window) = weak.upgrade() {
            window.set_priority_input(SharedString::new());
        }
    });

    let media_for_add = Rc::clone(&media_options);
    let callback_repository = Rc::clone(&repository);
    let weak = window.as_weak();
    window.on_add_media(move |value| {
        let value = value.trim();
        if value.is_empty()
            || media_for_add
                .iter()
                .any(|existing| existing.as_str().eq_ignore_ascii_case(value))
        {
            return;
        }
        if callback_repository
            .borrow_mut()
            .add_media_option(value)
            .is_err()
        {
            return;
        }
        media_for_add.push(value.into());
        if let Some(window) = weak.upgrade() {
            window.set_media_input(SharedString::new());
        }
    });

    let priorities_for_remove = Rc::clone(&priorities);
    let callback_repository = Rc::clone(&repository);
    window.on_remove_priority(move |index| {
        let index = index.max(0) as usize;
        if priorities_for_remove.row_count() > 1 && index < priorities_for_remove.row_count() {
            let Some(value) = priorities_for_remove.row_data(index) else {
                return;
            };
            if callback_repository
                .borrow_mut()
                .remove_priority_option(value.as_str())
                .is_err()
            {
                return;
            }
            priorities_for_remove.remove(index);
        }
    });

    let media_for_remove = Rc::clone(&media_options);
    let callback_repository = Rc::clone(&repository);
    window.on_remove_media(move |index| {
        let index = index.max(0) as usize;
        if media_for_remove.row_count() > 1 && index < media_for_remove.row_count() {
            let Some(value) = media_for_remove.row_data(index) else {
                return;
            };
            if callback_repository
                .borrow_mut()
                .remove_media_option(value.as_str())
                .is_err()
            {
                return;
            }
            media_for_remove.remove(index);
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_search_changed(move |value| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().query = value.to_string();
        if let Err(error) = refresh(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_tag_selected(move |value| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().tag = value.to_string();
        window.set_active_tag(value);
        if let Err(error) = refresh(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_sort_selected(move |index| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().sort_mode = index.max(0) as usize;
        if let Err(error) = refresh(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_show_hidden_changed(move |show_hidden| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().show_hidden = show_hidden;
        window.set_show_hidden(show_hidden);
        window.set_active_tag(SharedString::new());
        callback_state.borrow_mut().tag.clear();
        clear_editor(&window);
        if let Err(error) = refresh(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_save_server(
        move |id, name, ip_address, ports, environment, status, purpose, tags| {
            let Some(window) = weak.upgrade() else { return };
            let draft = ServerDraft {
                name: name.to_string(),
                tags: server_tags_from_input(tags.as_str()),
                ip_address: ip_address.to_string(),
                ports: ports.to_string(),
                environment: environment.to_string(),
                status: status.to_string(),
                purpose: purpose.to_string(),
            };
            let id = (id > 0).then_some(i64::from(id));
            let save_result = callback_repository.borrow_mut().save_server(id, &draft);
            match save_result {
                Ok(saved_id) => {
                    if let Err(error) = refresh(
                        &window,
                        &callback_repository.borrow(),
                        &callback_state.borrow(),
                    ) {
                        show_error(&window, &error);
                        return;
                    }
                    window.set_editing_id(i32::try_from(saved_id).unwrap_or(i32::MAX));
                    window.set_onboarding_open(false);
                    show_status(&window, "Server saved");
                }
                Err(error) => show_error(&window, &error),
            }
        },
    );

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_hide_server(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository.borrow().hide_server(i64::from(id)) {
            Ok(()) => {
                window.set_hide_target_id(0);
                clear_editor(&window);
                if let Err(error) = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                    return;
                }
                show_status(&window, "Server hidden");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    window.on_restore_server(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository.borrow().restore_server(i64::from(id)) {
            Ok(()) => {
                if let Err(error) = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                    return;
                }
                show_status(&window, "Server restored");
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_site_search_changed(move |value| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().query = value.to_string();
        if let Err(error) = refresh_sites(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_site_sort_selected(move |mode| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().sort_mode = mode.max(0) as usize;
        if let Err(error) = refresh_sites(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_show_hidden_sites_changed(move |show_hidden| {
        let Some(window) = weak.upgrade() else { return };
        callback_state.borrow_mut().show_hidden = show_hidden;
        window.set_show_hidden_sites(show_hidden);
        if let Err(error) = refresh_sites(
            &window,
            &callback_repository.borrow(),
            &callback_state.borrow(),
        ) {
            show_error(&window, &error);
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_save_site(move |id, name, url, environment, status, purpose, tags| {
        let Some(window) = weak.upgrade() else { return };
        let draft = SiteDraft {
            name: name.to_string(),
            url: url.to_string(),
            environment: environment.to_string(),
            status: status.to_string(),
            purpose: purpose.to_string(),
            tags: tags.split(',').map(str::to_owned).collect(),
        };
        let save_result = callback_repository
            .borrow_mut()
            .save_site((id > 0).then_some(i64::from(id)), &draft);
        match save_result {
            Ok(saved_id) => {
                window.set_editing_site_id(i32::try_from(saved_id).unwrap_or(i32::MAX));
                if let Err(error) = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                } else {
                    show_status(&window, "Site saved");
                }
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_hide_site(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository.borrow().hide_site(i64::from(id)) {
            Ok(()) => {
                window.set_hide_site_target_id(0);
                window.set_editing_site_id(0);
                if let Err(error) = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                } else {
                    show_status(&window, "Site hidden");
                }
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&site_table_state);
    let weak = window.as_weak();
    window.on_restore_site(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match callback_repository.borrow().restore_site(i64::from(id)) {
            Ok(()) => {
                if let Err(error) = refresh_sites(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                } else {
                    show_status(&window, "Site restored");
                }
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    window.on_copy_value(move |value, _label| {
        let Some(window) = weak.upgrade() else { return };
        match arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(value.to_string()))
        {
            Ok(()) => {
                window.set_copy_notice("Copied".into());
                let weak = window.as_weak();
                slint::Timer::single_shot(std::time::Duration::from_millis(1_600), move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_copy_notice(SharedString::new());
                    }
                });
            }
            Err(error) => show_error(&window, &error),
        }
    });

    let weak = window.as_weak();
    window.on_choose_import_file(move || {
        let Some(window) = weak.upgrade() else { return };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tables", &["xlsx", "xls", "xlsb", "ods", "csv", "tsv"])
            .pick_file()
        {
            window.set_import_file_path(path.to_string_lossy().into_owned().into());
        }
    });

    let weak = window.as_weak();
    window.on_save_import_template(move || {
        let Some(window) = weak.upgrade() else { return };
        let sites = window.get_importing_sites();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(if sites {
                "itgla-site-template.csv"
            } else {
                "itgla-server-template.csv"
            })
            .save_file()
        {
            match fs::write(
                &path,
                if sites {
                    SITE_IMPORT_TEMPLATE_CSV
                } else {
                    IMPORT_TEMPLATE_CSV
                },
            ) {
                Ok(()) => show_status(
                    &window,
                    if sites {
                        "Saved CSV template. Required columns: Name and URL."
                    } else {
                        "Saved CSV template. Required columns: IP Address and Ports."
                    },
                ),
                Err(error) => show_error(&window, &error),
            }
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_import = Rc::clone(&import_session);
    let callback_site_import = Rc::clone(&site_import_session);
    window.on_load_import(move |path| {
        let Some(window) = weak.upgrade() else { return };
        window.set_import_error(SharedString::new());
        match read_tabular(&PathBuf::from(path.as_str())) {
            Ok(data) if window.get_importing_sites() => {
                let mappings = automatic_site_mappings(&data);
                let column_rows = site_import_column_rows(&data, &mappings);
                window.set_import_mapping_options(ModelRc::new(VecModel::from(
                    site_mapping_options(),
                )));
                window.set_import_columns(ModelRc::new(VecModel::from(column_rows)));
                window.set_import_row_count(data.rows.len() as i32);
                window.set_import_ready(true);
                *callback_site_import.borrow_mut() = SiteImportSession {
                    data: Some(data),
                    mappings,
                };
            }
            Ok(data) => match callback_repository.borrow().server_columns() {
                Ok(columns) => {
                    let mappings = automatic_mappings(&data, &columns);
                    let column_rows = import_column_rows(&data, &mappings);
                    window.set_import_mapping_options(ModelRc::new(VecModel::from(
                        mapping_options(&columns),
                    )));
                    window.set_import_columns(ModelRc::new(VecModel::from(column_rows)));
                    window.set_import_row_count(data.rows.len() as i32);
                    window.set_import_ready(true);
                    *callback_import.borrow_mut() = ImportSession {
                        data: Some(data),
                        mappings,
                        existing_columns: columns,
                    };
                }
                Err(error) => window.set_import_error(error.to_string().into()),
            },
            Err(error) => window.set_import_error(error.to_string().into()),
        }
    });

    let weak = window.as_weak();
    let callback_import = Rc::clone(&import_session);
    let callback_site_import = Rc::clone(&site_import_session);
    window.on_auto_map_import(move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_importing_sites() {
            let mut session = callback_site_import.borrow_mut();
            let Some(data) = session.data.as_ref() else {
                return;
            };
            let mappings = automatic_site_mappings(data);
            let rows = site_import_column_rows(data, &mappings);
            session.mappings = mappings;
            window.set_import_columns(ModelRc::new(VecModel::from(rows)));
            window.set_import_error(SharedString::new());
            return;
        }
        let mut session = callback_import.borrow_mut();
        let Some(data) = session.data.as_ref() else {
            return;
        };
        let mappings = automatic_mappings(data, &session.existing_columns);
        let column_rows = import_column_rows(data, &mappings);
        session.mappings = mappings;
        window.set_import_columns(ModelRc::new(VecModel::from(column_rows)));
        window.set_import_error(SharedString::new());
    });

    let callback_import = Rc::clone(&import_session);
    let callback_site_import = Rc::clone(&site_import_session);
    let weak = window.as_weak();
    window.on_import_mapping_changed(move |source_index, mapping_index| {
        let Some(window) = weak.upgrade() else { return };
        if window.get_importing_sites() {
            let mut session = callback_site_import.borrow_mut();
            if let Some(mapping) = session.mappings.get_mut(source_index.max(0) as usize) {
                *mapping = mapping_index.max(0) as usize;
            }
            return;
        }
        let mut session = callback_import.borrow_mut();
        if let Some(mapping) = session.mappings.get_mut(source_index.max(0) as usize) {
            *mapping = mapping_index.max(0) as usize;
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    let callback_site_state = Rc::clone(&site_table_state);
    let callback_import = Rc::clone(&import_session);
    let callback_site_import = Rc::clone(&site_import_session);
    window.on_confirm_import(move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_importing_sites() {
            let session = callback_site_import.borrow();
            let Some(data) = session.data.as_ref() else {
                window.set_import_error("Load a file before importing".into());
                return;
            };
            let Some(targets) = site_import_targets(&session.mappings) else {
                window.set_import_error("A column mapping is no longer valid".into());
                return;
            };
            let defaults = SiteImportDefaults {
                environment: window.get_import_default_tags().to_string(),
                status: window.get_import_default_ip_address().to_string(),
                purpose: String::new(),
                tags: window.get_import_default_ports().to_string(),
                duplicate_policy: callback_repository
                    .borrow()
                    .workspace_preferences()
                    .import_duplicate_policy,
            };
            let import_result = callback_repository
                .borrow_mut()
                .import_sites(data, &targets, &defaults);
            match import_result {
                Ok(count) => {
                    drop(session);
                    if let Err(error) = refresh_sites(
                        &window,
                        &callback_repository.borrow(),
                        &callback_site_state.borrow(),
                    ) {
                        show_error(&window, &error);
                        return;
                    }
                    window.set_import_dialog_open(false);
                    window.set_import_ready(false);
                    show_status(&window, &format!("Imported {count} sites"));
                    *callback_site_import.borrow_mut() = SiteImportSession::default();
                }
                Err(error) => window.set_import_error(error.to_string().into()),
            }
            return;
        }
        let session = callback_import.borrow();
        let Some(data) = session.data.as_ref() else {
            window.set_import_error("Load a file before importing".into());
            return;
        };
        let targets = session
            .mappings
            .iter()
            .enumerate()
            .map(|(index, mapping)| import_target(index, *mapping, &session))
            .collect::<Option<Vec<_>>>();
        let Some(targets) = targets else {
            window.set_import_error("A column mapping is no longer valid".into());
            return;
        };
        let defaults = ServerImportDefaults {
            name: window.get_import_default_name().to_string(),
            tags: window.get_import_default_tags().to_string(),
            ip_address: window.get_import_default_ip_address().to_string(),
            ports: window.get_import_default_ports().to_string(),
            environment: window.get_import_default_environment().to_string(),
            status: window.get_import_default_status().to_string(),
            purpose: window.get_import_default_purpose().to_string(),
            duplicate_policy: callback_repository
                .borrow()
                .workspace_preferences()
                .import_duplicate_policy,
        };
        let import_result = callback_repository
            .borrow_mut()
            .import_servers_with_defaults(data, &targets, &defaults);
        match import_result {
            Ok(count) => {
                drop(session);
                if let Err(error) = refresh(
                    &window,
                    &callback_repository.borrow(),
                    &callback_state.borrow(),
                ) {
                    show_error(&window, &error);
                    return;
                }
                window.set_import_dialog_open(false);
                window.set_import_ready(false);
                window.set_onboarding_open(false);
                show_status(&window, &format!("Imported {count} servers"));
                *callback_import.borrow_mut() = ImportSession::default();
            }
            Err(error) => window.set_import_error(error.to_string().into()),
        }
    });

    window.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
    use slint::{
        LogicalPosition,
        platform::{Key, PointerEventButton, WindowEvent},
    };

    fn server(tags: &[&str], ip_address: &str, ports: &str, custom: &[&str]) -> ServerRecord {
        ServerRecord {
            id: 1,
            name: "server-01".into(),
            tags: tags.iter().map(|tag| (*tag).into()).collect(),
            ip_address: ip_address.into(),
            ports: ports.into(),
            environment: "production".into(),
            status: "active".into(),
            purpose: "Test server".into(),
            custom_values: custom.iter().map(|value| (*value).into()).collect(),
        }
    }

    #[test]
    fn combines_exact_tag_filter_with_free_text_search() {
        let record = server(
            &["production", "api"],
            "203.0.113.10",
            "22, 443",
            &["Platform"],
        );
        assert!(matches_filters(
            &record,
            &TableState {
                query: "platform".into(),
                tag: "PRODUCTION".into(),
                sort_mode: 0,
                show_hidden: false,
            }
        ));
        assert!(!matches_filters(
            &record,
            &TableState {
                query: "443".into(),
                tag: "database".into(),
                sort_mode: 0,
                show_hidden: false,
            }
        ));
    }

    #[test]
    fn default_sort_is_ascii_case_insensitive_and_other_modes_are_numeric() {
        let mut records = vec![
            server(&["beta"], "10.0.0.10", "443", &["west"]),
            server(&["Alpha"], "10.0.0.2", "22", &["east"]),
        ];
        records[0].name = "beta".into();
        records[1].name = "Alpha".into();
        sort_records(&mut records, 0);
        assert_eq!(records[0].name, "Alpha");
        sort_records(&mut records, 2);
        assert_eq!(records[0].ip_address, "10.0.0.2");
        sort_records(&mut records, 5);
        assert_eq!(records[0].ports, "443");
        sort_records(&mut records, 14);
        assert_eq!(records[0].custom_values, ["east"]);
    }

    #[test]
    fn custom_columns_sort_numbers_in_both_directions_and_keep_mixed_order_total() {
        let mut records = ["10", "2", "-1.5", "alpha", "Beta", "1a", "NaN", ""]
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let mut record = server(&["test"], "10.0.0.1", "22", &[value]);
                record.id = index as i64;
                record
            })
            .collect::<Vec<_>>();
        sort_records(&mut records, 14);
        let ascending = records
            .iter()
            .map(|r| r.custom_values[0].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ascending,
            ["-1.5", "2", "10", "", "1a", "alpha", "Beta", "NaN"]
        );
        sort_records(&mut records, 15);
        let descending = records
            .iter()
            .map(|r| r.custom_values[0].clone())
            .collect::<Vec<_>>();
        assert_eq!(descending, ascending.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn table_headers_toggle_sort_and_resize_only_while_pressed() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.window().set_size(PhysicalSize::new(1_000, 640));
        window.set_onboarding_open(false);
        assert_eq!(window.get_font_size_step(), 2);
        let weak = window.as_weak();
        window.on_sort_selected(move |mode| {
            weak.upgrade()
                .expect("live test window")
                .set_sort_mode_index(mode);
        });
        let weak = window.as_weak();
        window.on_column_width_changed(move |index, width| {
            update_column_width(
                &weak.upgrade().expect("live test window"),
                index as usize,
                width,
            );
        });
        window.show().expect("show test window");
        for expected in [2, 3, 2] {
            let position = LogicalPosition::new(250.0, 263.0);
            window
                .window()
                .dispatch_event(WindowEvent::PointerMoved { position });
            window.window().dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            window
                .window()
                .dispatch_event(WindowEvent::PointerReleased {
                    position,
                    button: PointerEventButton::Left,
                });
            assert_eq!(window.get_sort_mode_index(), expected);
        }
        let start = LogicalPosition::new(187.0, 263.0);
        window
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position: start });
        assert_eq!(window.get_column_widths().row_data(0), Some(160.0));
        window.window().dispatch_event(WindowEvent::PointerPressed {
            position: start,
            button: PointerEventButton::Left,
        });
        for x in [197.0, 207.0, 217.0] {
            window.window().dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(x, 263.0),
            });
        }
        let end = LogicalPosition::new(217.0, 263.0);
        window
            .window()
            .dispatch_event(WindowEvent::PointerReleased {
                position: end,
                button: PointerEventButton::Left,
            });
        assert_eq!(window.get_column_widths().row_data(0), Some(190.0));
        window.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(211.0, 263.0),
        });
        assert_eq!(window.get_column_widths().row_data(0), Some(190.0));
        assert_eq!(window.get_sort_mode_index(), 2, "resizing must not sort");
    }

    #[test]
    fn navigation_is_keyboard_traversable_in_both_directions() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        let weak = window.as_weak();
        window.on_tab_selected(move |tab| {
            let window = weak.upgrade().expect("live test window");
            if tab == 3 {
                window.set_work_page_scroll_y(0.0);
            }
            window.set_active_tab(tab);
        });
        window.show().expect("show test window");

        window.invoke_focus_navigation();
        for expected_tab in 0..=1 {
            window.window().dispatch_event(WindowEvent::KeyPressed {
                text: Key::Return.into(),
            });
            window.window().dispatch_event(WindowEvent::KeyReleased {
                text: Key::Return.into(),
            });
            assert_eq!(window.get_active_tab(), expected_tab);
            if expected_tab < 1 {
                window.window().dispatch_event(WindowEvent::KeyPressed {
                    text: Key::Tab.into(),
                });
                window.window().dispatch_event(WindowEvent::KeyReleased {
                    text: Key::Tab.into(),
                });
            }
        }

        window.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Backtab.into(),
        });
        window.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Backtab.into(),
        });
        window.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Return.into(),
        });
        window.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Return.into(),
        });
        assert_eq!(window.get_active_tab(), 0);
    }

    #[test]
    fn custom_navigation_exposes_screen_reader_names_roles_and_actions() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        let weak = window.as_weak();
        window.on_tab_selected(move |tab| {
            let window = weak.upgrade().expect("live test window");
            if tab == 3 {
                window.set_work_page_scroll_y(0.0);
            }
            window.set_active_tab(tab);
        });
        window.show().expect("show test window");

        let tab_elements = window
            .root_element()
            .query_descendants()
            .match_accessible_role(AccessibleRole::Tab)
            .find_all();
        let labels = tab_elements
            .iter()
            .filter_map(|element| element.accessible_label())
            .map(|label| label.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            ["Servers", "Sites", "Services & Domains", "Settings"]
        );

        let sites_tab = ElementHandle::find_by_accessible_label(&window, "Sites")
            .next()
            .expect("Sites tab in accessibility tree");
        sites_tab.invoke_accessible_default_action();
        assert_eq!(window.get_active_tab(), 1);
        let settings_tab = ElementHandle::find_by_accessible_label(&window, "Settings")
            .next()
            .expect("Settings tab in accessibility tree");
        settings_tab.invoke_accessible_default_action();
        assert_eq!(window.get_active_tab(), 2);
    }

    #[test]
    fn settings_save_dispatches_all_preferences_at_minimum_window_size() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.set_active_tab(2);
        window.window().set_size(PhysicalSize::new(1_000, 640));
        window.set_theme_index(1);
        window.set_font_size_step(2);
        window.set_table_density(0);
        window.set_show_cell_tooltips(false);
        window.set_fit_columns_on_open(true);
        window.set_import_duplicate_policy(2);
        window.set_backup_retention(14);
        let saved = Rc::new(RefCell::new(None));
        let observed = Rc::clone(&saved);
        window.on_preferences_changed(
            move |theme, font, density, hover, fit, duplicate, retention| {
                *observed.borrow_mut() =
                    Some((theme, font, density, hover, fit, duplicate, retention));
            },
        );
        window.show().expect("show settings");
        for label in [
            "Color: Field Atlas",
            "Text size: Large",
            "Row spacing: Comfy",
        ] {
            assert!(
                ElementHandle::find_by_accessible_label(&window, label)
                    .next()
                    .is_some()
            );
        }
        let save = ElementHandle::find_by_accessible_label(&window, "Save settings")
            .next()
            .expect("persistent save action");
        save.invoke_accessible_default_action();
        assert_eq!(*saved.borrow(), Some((1, 2, 0, false, true, 2, 14)));
    }

    #[test]
    fn appearance_choices_preview_without_saving_and_keep_one_selection() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.set_active_tab(2);
        window.window().set_size(PhysicalSize::new(1_000, 800));
        window.show().expect("show settings");
        let saved = Rc::new(RefCell::new(None));
        let observed = Rc::clone(&saved);
        window.on_preferences_changed(move |theme, font, density, _, _, _, _| {
            *observed.borrow_mut() = Some((theme, font, density));
        });
        for label in ["Color: Sand", "Text size: Small", "Row spacing: Roomy"] {
            let choice = ElementHandle::find_by_accessible_label(&window, label)
                .next()
                .expect("visible appearance choice");
            choice.invoke_accessible_default_action();
            choice.invoke_accessible_default_action();
            assert_eq!(choice.accessible_checked(), Some(true));
        }
        assert_eq!(window.get_theme_index(), 2);
        assert_eq!(window.get_font_size_step(), 0);
        assert_eq!(window.get_table_density(), 2);
        assert!(window.get_appearance_dirty());
        assert_eq!(*saved.borrow(), None, "preview must not silently persist");
        let old_theme = ElementHandle::find_by_accessible_label(&window, "Color: Field Atlas")
            .next()
            .expect("previous theme");
        assert_eq!(old_theme.accessible_checked(), Some(false));
        ElementHandle::find_by_accessible_label(&window, "Save settings")
            .next()
            .expect("save button")
            .invoke_accessible_default_action();
        assert_eq!(*saved.borrow(), Some((2, 0, 2)));
        assert!(!window.get_appearance_dirty());
    }

    #[test]
    fn failed_preferences_save_keeps_preview_and_exposes_error() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.set_active_tab(2);
        window.window().set_size(PhysicalSize::new(1_000, 800));
        window.show().expect("show settings");
        let weak = window.as_weak();
        window.on_preferences_changed(move |_, _, _, _, _, _, _| {
            let window = weak.upgrade().expect("live window");
            window.set_status_is_error(true);
            window.set_status_message("Preferences could not be saved".into());
        });
        ElementHandle::find_by_accessible_label(&window, "Color: Sand")
            .next()
            .expect("color choice")
            .invoke_accessible_default_action();
        ElementHandle::find_by_accessible_label(&window, "Save settings")
            .next()
            .expect("save button")
            .invoke_accessible_default_action();
        assert_eq!(window.get_theme_index(), 2);
        assert!(window.get_appearance_dirty());
        assert!(window.get_appearance_error());
        assert!(
            ElementHandle::find_by_accessible_label(&window, "Preferences could not be saved")
                .next()
                .is_some(),
            "error must take priority over the unsaved-preview hint"
        );
    }

    #[test]
    fn disabled_action_rejects_accessibility_activation() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.set_active_tab(2);
        let calls = Rc::new(RefCell::new(0));
        let observed = Rc::clone(&calls);
        window.on_auto_fit_columns(move || *observed.borrow_mut() += 1);
        window.show().expect("show window");
        let action = ElementHandle::find_by_accessible_label(&window, "Fit columns")
            .next()
            .expect("disabled header action");
        assert_eq!(action.accessible_enabled(), Some(false));
        action.invoke_accessible_default_action();
        assert_eq!(*calls.borrow(), 0);
    }

    #[test]
    fn screen_reader_tree_covers_server_and_site_tables() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.set_servers(ModelRc::new(VecModel::from(vec![ServerRow {
            id: 7,
            name: "api-07".into(),
            tags: "production".into(),
            ip_address: "203.0.113.7".into(),
            ports: "22, 443".into(),
            environment: "production".into(),
            status: "active".into(),
            purpose: "Public API".into(),
            custom_values: ModelRc::new(VecModel::from(Vec::<SharedString>::new())),
        }])));
        window.set_available_tags(ModelRc::new(VecModel::from(vec!["production".into()])));
        window.show().expect("show test window");

        assert!(
            ElementHandle::find_by_accessible_label(&window, "Name: api-07")
                .next()
                .is_some(),
            "server cells need a screen-reader name"
        );
        assert!(
            ElementHandle::find_by_accessible_label(&window, "NAME, sorted ascending")
                .next()
                .is_some(),
            "sortable table headers need their direction in the name"
        );
        assert!(
            ElementHandle::find_by_accessible_label(&window, "Filter by production")
                .next()
                .is_some(),
            "tag filters need a screen-reader name"
        );

        window.set_sites(ModelRc::new(VecModel::from(vec![SiteRow {
            id: 11,
            name: "Customer portal".into(),
            url: "https://app.example.com".into(),
            environment: "production".into(),
            status: "active".into(),
            purpose: "Customer application".into(),
            tags: "customer-facing".into(),
        }])));
        window.set_active_tab(1);
        assert!(
            ElementHandle::find_by_accessible_label(&window, "Name: Customer portal")
                .next()
                .is_some(),
            "site cells need a screen-reader name"
        );
        assert!(
            ElementHandle::find_by_accessible_label(&window, "URL: https://app.example.com")
                .next()
                .is_some(),
            "site URLs need a screen-reader name"
        );
    }

    #[test]
    fn desktop_layout_accepts_windows_high_dpi_scale_factors() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.set_onboarding_open(false);
        window.window().set_size(PhysicalSize::new(1_500, 960));
        window.show().expect("show test window");

        for scale_factor in [1.25_f32, 1.5, 2.0] {
            window
                .window()
                .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
            assert!((window.window().scale_factor() - scale_factor).abs() < f32::EPSILON);
            assert_eq!(window.get_font_size_step(), 2);
            assert_eq!(window.get_work_page_scroll_y(), 0.0);
        }
    }

    #[test]
    fn table_geometry_counts_each_column_width_once() {
        let default_widths = [180.0, 160.0, 130.0];
        assert_eq!(table_data_width(&default_widths), 470.0);
        assert_eq!(table_width_for(&default_widths), 816.0);

        let fitted_widths = [96.0, 112.0, 96.0];
        assert_eq!(table_data_width(&fitted_widths), 304.0);
        assert_eq!(table_width_for(&fitted_widths), 816.0);

        let wide_widths = [420.0, 420.0, 420.0, 300.0];
        assert_eq!(table_data_width(&wide_widths), 1_560.0);
        assert_eq!(table_width_for(&wide_widths), 1_676.0);
    }

    #[test]
    fn table_geometry_stays_bounded_for_dense_and_extreme_shapes() {
        for custom_count in [0_usize, 1, 8, 32] {
            let headers = (0..custom_count)
                .map(|index| format!("Custom field {index}"))
                .collect::<Vec<_>>();
            let mut row = vec![
                "api-01".into(),
                "203.0.113.10".into(),
                "22, 443".into(),
                "production".into(),
                "active".into(),
                "Public API".into(),
                "api, customer-facing".into(),
            ];
            row.extend((0..custom_count).map(|index| {
                if index == custom_count.saturating_sub(1) {
                    "x".repeat(4_096)
                } else {
                    format!("value-{index}")
                }
            }));
            let rows = vec![row];
            let widths = fitted_column_widths(&headers, &rows);
            assert_eq!(widths.len(), 7 + custom_count);
            assert!(widths.iter().all(|width| (96.0..=420.0).contains(width)));
            assert!(table_width_for(&widths) >= 816.0);
            assert_eq!(
                table_width_for(&widths),
                (table_data_width(&widths) + 116.0).max(816.0)
            );
            assert_eq!(widths, fitted_column_widths(&headers, &rows));
        }
    }

    #[test]
    fn server_fields_accept_pointer_positioned_edits() {
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().expect("test window");
        window.window().set_size(PhysicalSize::new(1_000, 640));
        window.set_active_tab(0);
        window.set_onboarding_open(false);
        window.set_editing_ip_address("203.0.113.10".into());
        window.show().expect("show test window");

        let position = LogicalPosition::new(710.0, 293.0);
        window
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        window.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        window
            .window()
            .dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
        window
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: "X".into() });
        window
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text: "X".into() });

        let edited = window.get_editing_ip_address().to_string();
        assert_eq!(
            edited.chars().filter(|character| *character == 'X').count(),
            1
        );
        assert!(
            !edited.starts_with('X'),
            "cursor unexpectedly stayed at the start: {edited}"
        );
        assert!(
            !edited.ends_with('X'),
            "cursor unexpectedly moved to the end: {edited}"
        );
    }

    #[test]
    fn maps_known_headers_to_required_and_suggested_fields() {
        let columns = vec![ServerColumn {
            id: 7,
            name: "Owner".to_owned(),
            value_type: "text".to_owned(),
            default_value: String::new(),
            hidden: false,
        }];
        assert_eq!(default_mapping("IP Address", &columns), 2);
        assert_eq!(default_mapping("Ports", &columns), 3);
        assert_eq!(default_mapping("Environment", &columns), 4);
        assert_eq!(
            default_mapping("Cloud Provider", &columns),
            8 + columns.len() + 1
        );
        assert_eq!(default_mapping("Owner", &columns), 8);
        assert_eq!(
            default_mapping("Unrecognized Column", &columns),
            8 + columns.len() + SUGGESTED_CUSTOM_FIELDS.len()
        );
    }

    #[test]
    fn auto_mapping_uses_headers_and_sample_values_without_duplicates() {
        let data = TabularData {
            headers: vec!["Server".into(), "Public IP".into(), "Service".into()],
            rows: vec![vec!["api".into(), "203.0.113.10".into(), "22,443".into()]],
        };
        assert_eq!(
            automatic_mappings(&data, &[]),
            [8 + SUGGESTED_CUSTOM_FIELDS.len(), 2, 3]
        );
    }

    #[test]
    fn resolves_suggested_and_source_named_custom_fields() {
        let session = ImportSession {
            data: Some(TabularData {
                headers: vec!["Business Unit".to_owned()],
                rows: vec![vec!["Platform".to_owned()]],
            }),
            mappings: Vec::new(),
            existing_columns: vec![ServerColumn {
                id: 9,
                name: "Owner".to_owned(),
                value_type: "text".to_owned(),
                default_value: String::new(),
                hidden: false,
            }],
        };
        assert_eq!(
            import_target(0, 8, &session),
            Some(ServerImportTarget::ExistingCustom(9))
        );
        assert_eq!(
            import_target(0, 8 + session.existing_columns.len() + 4, &session),
            Some(ServerImportTarget::NewCustom("Project".to_owned()))
        );
        assert_eq!(
            import_target(
                0,
                8 + session.existing_columns.len() + SUGGESTED_CUSTOM_FIELDS.len(),
                &session,
            ),
            Some(ServerImportTarget::NewCustom("Business Unit".to_owned()))
        );
    }
}

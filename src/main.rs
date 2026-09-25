#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[allow(dead_code)]
mod domain;
mod import_data;
#[allow(dead_code)]
mod storage;

use std::{cell::RefCell, cmp::Ordering, net::IpAddr, path::PathBuf, rc::Rc};

use domain::{ServerColumn, ServerDraft, ServerRecord, server_tags_from_input};
use import_data::{TabularData, read_tabular};
use slint::{ModelRc, SharedString, VecModel};
use storage::{Repository, ServerImportTarget, StorageError};

slint::include_modules!();

#[derive(Default)]
struct TableState {
    query: String,
    tag: String,
    sort_mode: usize,
    show_hidden: bool,
}

#[derive(Default)]
struct ImportSession {
    data: Option<TabularData>,
    mappings: Vec<usize>,
    existing_columns: Vec<ServerColumn>,
}

fn to_row(server: &ServerRecord) -> ServerRow {
    ServerRow {
        id: i32::try_from(server.id).unwrap_or(i32::MAX),
        tags: server.tags.join(", ").into(),
        ip_address: server.ip_address.as_str().into(),
        ports: server.ports.as_str().into(),
        custom_values: ModelRc::new(VecModel::from(
            server
                .custom_values
                .iter()
                .map(SharedString::from)
                .collect::<Vec<_>>(),
        )),
    }
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
        || server
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&query))
        || server.ip_address.to_lowercase().contains(&query)
        || server.ports.to_lowercase().contains(&query)
        || server
            .custom_values
            .iter()
            .any(|value| value.to_lowercase().contains(&query));
    matches_tag && matches_query
}

fn sort_records(records: &mut [ServerRecord], mode: usize) {
    records.sort_by(|left, right| {
        let (ordering, ascending) = match mode {
            0 => (
                text_compare(&left.tags.join(", "), &right.tags.join(", ")),
                true,
            ),
            1 => (
                text_compare(&left.tags.join(", "), &right.tags.join(", ")),
                false,
            ),
            2 => (ip_compare(&left.ip_address, &right.ip_address), true),
            3 => (ip_compare(&left.ip_address, &right.ip_address), false),
            4 => (first_port(&left.ports).cmp(&first_port(&right.ports)), true),
            5 => (
                first_port(&left.ports).cmp(&first_port(&right.ports)),
                false,
            ),
            6 => (left.id.cmp(&right.id), false),
            7 => (left.id.cmp(&right.id), true),
            custom => {
                let column_index = (custom - 8) / 2;
                let ascending = (custom - 8).is_multiple_of(2);
                let left_value = left
                    .custom_values
                    .get(column_index)
                    .map_or("", String::as_str);
                let right_value = right
                    .custom_values
                    .get(column_index)
                    .map_or("", String::as_str);
                (text_compare(left_value, right_value), ascending)
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
    let mut records = if state.show_hidden {
        hidden_records.clone()
    } else {
        repository.server_records()?
    };
    let mut tags = records
        .iter()
        .flat_map(|server| server.tags.iter().cloned())
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_lowercase());
    tags.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    sort_records(&mut records, state.sort_mode);
    let rows = records
        .iter()
        .filter(|server| matches_filters(server, state))
        .map(to_row)
        .collect::<Vec<_>>();

    window.set_server_count(rows.len() as i32);
    window.set_hidden_count(hidden_records.len() as i32);
    window.set_servers(ModelRc::new(VecModel::from(rows)));
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
    Ok(())
}

fn mapping_options(columns: &[ServerColumn]) -> Vec<SharedString> {
    let mut options = vec![
        "Skip".into(),
        "Tags".into(),
        "IP Address".into(),
        "Ports".into(),
    ];
    options.extend(
        columns
            .iter()
            .map(|column| SharedString::from(format!("Existing: {}", column.name))),
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
    if matches!(normalized.as_str(), "tag" | "tags" | "label" | "labels") {
        return 1;
    }
    if matches!(normalized.as_str(), "ip" | "ipaddress" | "address" | "host") {
        return 2;
    }
    if matches!(normalized.as_str(), "port" | "ports" | "openports") {
        return 3;
    }
    if let Some(index) = columns
        .iter()
        .position(|column| column.name.eq_ignore_ascii_case(header.trim()))
    {
        return 4 + index;
    }
    4 + columns.len()
}

fn import_target(
    source_index: usize,
    mapping_index: usize,
    session: &ImportSession,
) -> Option<ServerImportTarget> {
    match mapping_index {
        0 => Some(ServerImportTarget::Skip),
        1 => Some(ServerImportTarget::Tags),
        2 => Some(ServerImportTarget::IpAddress),
        3 => Some(ServerImportTarget::Ports),
        index if index < 4 + session.existing_columns.len() => session
            .existing_columns
            .get(index - 4)
            .map(|column| ServerImportTarget::ExistingCustom(column.id)),
        index if index == 4 + session.existing_columns.len() => session
            .data
            .as_ref()
            .and_then(|data| data.headers.get(source_index))
            .map(|header| ServerImportTarget::NewCustom(header.clone())),
        _ => None,
    }
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
    window.set_editing_tags(SharedString::new());
    window.set_editing_ip_address(SharedString::new());
    window.set_editing_ports(SharedString::new());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Rc::new(RefCell::new(Repository::open_default()?));
    let table_state = Rc::new(RefCell::new(TableState::default()));
    let import_session = Rc::new(RefCell::new(ImportSession::default()));
    let window = AppWindow::new()?;
    refresh(&window, &repository.borrow(), &table_state.borrow())?;

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
    window.on_save_server(move |id, tags, ip_address, ports| {
        let Some(window) = weak.upgrade() else { return };
        let draft = ServerDraft {
            tags: server_tags_from_input(tags.as_str()),
            ip_address: ip_address.to_string(),
            ports: ports.to_string(),
        };
        let id = (id > 0).then_some(i64::from(id));
        match callback_repository.borrow_mut().save_server(id, &draft) {
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
                show_status(&window, "Server saved");
            }
            Err(error) => show_error(&window, &error),
        }
    });

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
    let callback_repository = Rc::clone(&repository);
    let callback_import = Rc::clone(&import_session);
    window.on_load_import(move |path| {
        let Some(window) = weak.upgrade() else { return };
        window.set_import_error(SharedString::new());
        match read_tabular(&PathBuf::from(path.as_str())) {
            Ok(data) => match callback_repository.borrow().server_columns() {
                Ok(columns) => {
                    let mappings = data
                        .headers
                        .iter()
                        .map(|header| default_mapping(header, &columns))
                        .collect::<Vec<_>>();
                    let column_rows = data
                        .headers
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
                        .collect::<Vec<_>>();
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

    let callback_import = Rc::clone(&import_session);
    window.on_import_mapping_changed(move |source_index, mapping_index| {
        let mut session = callback_import.borrow_mut();
        if let Some(mapping) = session.mappings.get_mut(source_index.max(0) as usize) {
            *mapping = mapping_index.max(0) as usize;
        }
    });

    let weak = window.as_weak();
    let callback_repository = Rc::clone(&repository);
    let callback_state = Rc::clone(&table_state);
    let callback_import = Rc::clone(&import_session);
    window.on_confirm_import(move || {
        let Some(window) = weak.upgrade() else { return };
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
        match callback_repository
            .borrow_mut()
            .import_servers(data, &targets)
        {
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

    fn server(tags: &[&str], ip_address: &str, ports: &str, custom: &[&str]) -> ServerRecord {
        ServerRecord {
            id: 1,
            tags: tags.iter().map(|tag| (*tag).into()).collect(),
            ip_address: ip_address.into(),
            ports: ports.into(),
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
        sort_records(&mut records, 0);
        assert_eq!(records[0].tags, ["Alpha"]);
        sort_records(&mut records, 2);
        assert_eq!(records[0].ip_address, "10.0.0.2");
        sort_records(&mut records, 5);
        assert_eq!(records[0].ports, "443");
        sort_records(&mut records, 8);
        assert_eq!(records[0].custom_values, ["east"]);
    }
}

//! Saved-resource topology and the isolated, complete demonstration workspace.
use crate::{
    AppWindow, BusinessItem, TopologyEdge, TopologyNode,
    operational::Details,
    storage::{Repository, StorageError},
};
use rusqlite::params;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

const RELATIONS: [&str; 10] = [
    "owns",
    "contains",
    "resolves_to",
    "hosted_on",
    "origin_on",
    "depends_on",
    "replicates_to",
    "fails_over_to",
    "balances_to",
    "related_to",
];
fn invalid(text: &str) -> StorageError {
    StorageError::InvalidImport(text.into())
}
#[derive(Clone)]
struct Resource {
    id: i64,
    kind: String,
    label: String,
    detail: String,
}
fn resources(repo: &Repository) -> Result<Vec<Resource>, StorageError> {
    let mut out = Vec::new();
    for b in repo.businesses()? {
        out.push(Resource {
            id: b.id,
            kind: "business".into(),
            label: b.name,
            detail: b.tags,
        });
    }
    for e in repo
        .inventory_entries()?
        .into_iter()
        .filter(|e| !e.hidden && e.kind == 1)
    {
        out.push(Resource {
            id: e.id,
            kind: "domain".into(),
            label: e.details.name,
            detail: if e.details.parent.is_empty() {
                e.details.cdn
            } else {
                format!("Subdomain of {}", e.details.parent)
            },
        });
    }
    for s in repo.server_records()? {
        out.push(Resource {
            id: s.id,
            kind: "server".into(),
            label: s.name,
            detail: s.ip_address,
        });
    }
    for e in repo
        .inventory_entries()?
        .into_iter()
        .filter(|e| !e.hidden && e.kind == 0)
    {
        out.push(Resource {
            id: e.id,
            kind: "service".into(),
            label: e.details.name,
            detail: format!("{} / {}", e.details.protocol, e.details.ports),
        });
    }
    // Parent domains precede children; services align with their hosting servers.
    let entries = repo.inventory_entries()?;
    let server_ids = out
        .iter()
        .filter(|r| r.kind == "server")
        .map(|r| r.id)
        .collect::<Vec<_>>();
    out.sort_by_key(|r| {
        let entry = entries
            .iter()
            .find(|e| e.id == r.id && matches!(r.kind.as_str(), "domain" | "service"));
        let rank = match r.kind.as_str() {
            "business" => 0,
            "domain" => 1,
            "server" => 2,
            _ => 3,
        };
        let order = if r.kind == "service" {
            entry
                .and_then(|e| {
                    e.hosts
                        .iter()
                        .filter_map(|id| server_ids.iter().position(|s| s == id))
                        .min()
                })
                .unwrap_or(usize::MAX)
        } else if r.kind == "domain" {
            usize::from(entry.is_some_and(|e| !e.details.parent.is_empty()))
        } else {
            0
        };
        (rank, order)
    });
    Ok(out)
}
fn key(kind: &str, id: i64) -> (String, i64) {
    (kind.into(), id)
}
fn relation_name(value: &str) -> String {
    value.replace('_', " ")
}

pub fn refresh(w: &AppWindow, repo: &Repository) -> Result<(), StorageError> {
    let mut stmt=repo.connection.prepare("SELECT id,name,tags,notes FROM businesses WHERE (archived_at IS NOT NULL)=?1 ORDER BY lower(name),id LIMIT 1000")?;
    let businesses = stmt
        .query_map([w.get_business_hidden()], |r| {
            Ok(BusinessItem {
                id: r.get::<_, i64>(0)? as i32,
                name: r.get::<_, String>(1)?.into(),
                tags: r.get::<_, String>(2)?.into(),
                notes: r.get::<_, String>(3)?.into(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    w.set_business_rows(ModelRc::new(VecModel::from(businesses)));
    let all = resources(repo)?;
    w.set_topology_choices(ModelRc::new(VecModel::from(
        all.iter()
            .map(|r| format!("{}: {}", r.kind, r.label).into())
            .collect::<Vec<_>>(),
    )));
    let active_businesses = repo.businesses()?;
    let mut labels = vec!["All businesses".into()];
    labels.extend(active_businesses.iter().map(|b| b.name.clone().into()));
    if w.get_topology_filter() < 0 || w.get_topology_filter() as usize >= labels.len() {
        w.set_topology_filter(0);
    }
    w.set_topology_business_choices(ModelRc::new(VecModel::from(labels)));
    let links = repo.resource_links()?;
    let active = all
        .iter()
        .map(|r| key(&r.kind, r.id))
        .collect::<BTreeSet<_>>();
    let mut included = active.clone();
    if w.get_topology_filter() > 0 {
        included.clear();
        if let Some(b) = active_businesses.get(w.get_topology_filter() as usize - 1) {
            included.insert(key("business", b.id));
        }
        // Traverse ownership and hosting downstream; cross-business dependencies stay explicit.
        loop {
            let old = included.len();
            for l in &links {
                if included.contains(&key(&l.source_type, l.source_id))
                    && active.contains(&key(&l.target_type, l.target_id))
                {
                    included.insert(key(&l.target_type, l.target_id));
                }
                if l.relation_type == "hosted_on"
                    && included.contains(&key(&l.target_type, l.target_id))
                    && active.contains(&key(&l.source_type, l.source_id))
                {
                    included.insert(key(&l.source_type, l.source_id));
                }
            }
            if included.len() == old {
                break;
            }
        }
    }
    let total = all
        .iter()
        .filter(|r| included.contains(&key(&r.kind, r.id)))
        .count();
    let visible = all
        .iter()
        .filter(|r| included.contains(&key(&r.kind, r.id)))
        .take(200)
        .collect::<Vec<_>>();
    let mut counters = [0usize; 4];
    let mut locations = HashMap::new();
    let mut nodes = Vec::new();
    for r in visible {
        let col = match r.kind.as_str() {
            "business" => 0,
            "domain" => 1,
            "server" => 2,
            _ => 3,
        };
        let x = 20.0 + col as f32 * 300.0;
        let y = 48.0 + counters[col] as f32 * 112.0;
        counters[col] += 1;
        locations.insert(key(&r.kind, r.id), (x, y));
        nodes.push(TopologyNode {
            id: r.id as i32,
            kind: r.kind.clone().into(),
            label: r.label.clone().into(),
            detail: r.detail.clone().into(),
            px: x,
            py: y,
        });
    }
    let resource_labels: HashMap<_, _> = all
        .iter()
        .map(|r| (key(&r.kind, r.id), r.label.as_str()))
        .collect();
    // Assign distinct ports on each node side and separate tracks in each gutter.
    // Long links cross intervening lanes only in the empty space between rows.
    let routed = links
        .iter()
        .filter_map(|l| {
            let &(sx, sy) = locations.get(&key(&l.source_type, l.source_id))?;
            let &(tx, ty) = locations.get(&key(&l.target_type, l.target_id))?;
            let sc = ((sx - 20.0) / 300.0) as usize;
            let tc = ((tx - 20.0) / 300.0) as usize;
            let sr = sc <= tc;
            let tr = sc >= tc;
            Some((
                l,
                sx,
                sy,
                tx,
                ty,
                sr,
                tr,
                if sr { sc } else { sc - 1 },
                if tr { tc } else { tc - 1 },
            ))
        })
        .collect::<Vec<_>>();
    let mut ports = HashMap::new();
    let mut gutters = HashMap::new();
    let mut rails = HashMap::new();
    for (l, _, sy, _, _, sr, tr, sg, tg) in &routed {
        *ports
            .entry((key(&l.source_type, l.source_id), *sr))
            .or_insert(0usize) += 1;
        *ports
            .entry((key(&l.target_type, l.target_id), *tr))
            .or_insert(0usize) += 1;
        *gutters.entry(*sg).or_insert(0usize) += 1;
        if sg != tg {
            *gutters.entry(*tg).or_insert(0usize) += 1;
            *rails.entry(*sy as i32).or_insert(0usize) += 1;
        }
    }
    let mut used_ports = HashMap::new();
    let mut used_gutters = HashMap::new();
    let mut used_rails = HashMap::new();
    let mut edges = Vec::new();
    for (l, sx, sy, tx, ty, sr, tr, sg, tg) in routed {
        let mut port = |kind: &str, id, side| {
            let k = (key(kind, id), side);
            let i = used_ports.entry(k.clone()).or_insert(0usize);
            *i += 1;
            8.0 + *i as f32 * 50.0 / (ports[&k] + 1) as f32
        };
        let source_y = sy + port(&l.source_type, l.source_id, sr);
        let target_y = ty + port(&l.target_type, l.target_id, tr);
        let mut track = |gap| {
            let i = used_gutters.entry(gap).or_insert(0usize);
            *i += 1;
            20.0 + gap as f32 * 300.0 + 220.0 + 8.0 + *i as f32 * 64.0 / (gutters[&gap] + 1) as f32
        };
        let mx = track(sg);
        let mx2 = if sg == tg { mx } else { track(tg) };
        let route_y = if sg == tg {
            target_y
        } else {
            let row = sy as i32;
            let i = used_rails.entry(row).or_insert(0usize);
            *i += 1;
            sy + 72.0 + *i as f32 * 34.0 / (rails[&row] + 1) as f32
        };
        edges.push(TopologyEdge {
            id: l.id as i32,
            label: format!(
                "{} -> {} -> {}",
                resource_labels[&key(&l.source_type, l.source_id)],
                relation_name(&l.relation_type),
                resource_labels[&key(&l.target_type, l.target_id)]
            )
            .into(),
            structural: matches!(
                l.relation_type.as_str(),
                "hosted_on" | "origin_on" | "contains"
            ) || (l.relation_type == "owns"
                && l.target_type == "domain"
                && !links
                    .iter()
                    .any(|p| p.relation_type == "contains" && p.target_id == l.target_id)),
            source_key: format!("{}:{}", l.source_type, l.source_id).into(),
            target_key: format!("{}:{}", l.target_type, l.target_id).into(),
            caption: relation_name(&l.relation_type).into(),
            sx: sx + if sr { 220.0 } else { 0.0 },
            sy: source_y,
            tx: tx + if tr { 220.0 } else { 0.0 },
            ty: target_y,
            mx,
            mx2,
            route_y,
            reverse: tr,
            editable: true,
        });
    }
    w.set_topology_height(80.0 + counters.into_iter().max().unwrap_or(0) as f32 * 112.0);
    w.set_topology_node_choices(ModelRc::new(VecModel::from(
        nodes
            .iter()
            .map(|n| format!("{}: {}", n.kind, n.label).into())
            .collect::<Vec<_>>(),
    )));
    w.set_topology_nodes(ModelRc::new(VecModel::from(nodes)));
    w.set_topology_edges(ModelRc::new(VecModel::from(edges)));
    w.set_topology_status(if total>200 {format!("Showing 200 of {total} resources. Filter by business to narrow the map.").into()}else{format!("{total} active resources. Arrows show relationship direction. All links includes dependencies.").into()});
    Ok(())
}
fn report(w: &AppWindow, result: Result<(), StorageError>) {
    if let Err(e) = result {
        w.set_topology_status(e.to_string().into());
    }
}
/// Selections must not carry over between the real and in-memory workspaces.
pub fn clear_selection(w: &AppWindow) {
    w.set_business_id(0);
    w.set_business_name("".into());
    w.set_business_tags("".into());
    w.set_business_notes("".into());
    w.set_business_hidden(false);
    w.set_topology_filter(0);
    w.set_topology_link(0);
    w.set_topology_notes("".into());
    w.set_topology_source(0);
    w.set_topology_target(0);
    w.set_topology_relation(0);
}
pub fn setup(window: &AppWindow, repo: Rc<RefCell<Repository>>) {
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_topology_refresh(move || {
        if let Some(w) = weak.upgrade() {
            report(&w, refresh(&w, &r.borrow()));
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_business_select(move |id| {
        let Some(w) = weak.upgrade() else { return };
        w.set_business_id(id);
        w.set_business_name("".into());
        w.set_business_tags("".into());
        w.set_business_notes("".into());
        let result = (|| {
            let r = r.borrow();
            let mut stmt = r
                .connection
                .prepare("SELECT name,tags,notes FROM businesses WHERE id=?1")?;
            if id != 0 {
                let (name, tags, notes) = stmt.query_row([id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?;
                w.set_business_name(name.into());
                w.set_business_tags(tags.into());
                w.set_business_notes(notes.into());
            }
            if id != 0
                && let Some(event) = r.resource_history("business", i64::from(id))?.first()
            {
                w.set_topology_status(
                    format!(
                        "{} | {} | {}",
                        event.occurred_at, event.action, event.summary
                    )
                    .into(),
                );
            }
            Ok(())
        })();
        report(&w, result);
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_business_save(move ||{let Some(w)=weak.upgrade()else{return};let result=(||{let id=r.borrow_mut().save_business(i64::from(w.get_business_id()),w.get_business_name().as_str(),w.get_business_tags().as_str(),w.get_business_notes().as_str())?;w.set_business_id(id as i32);refresh(&w,&r.borrow())?;w.set_topology_status("Business saved. Assign it in a domain or service, or add an Owns link in Connections.".into());Ok(())})();report(&w,result);});
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_business_hide(move ||{let Some(w)=weak.upgrade()else{return};let result=(||{let n=r.borrow().connection.execute("UPDATE businesses SET archived_at=CASE WHEN ?1 THEN NULL ELSE CURRENT_TIMESTAMP END WHERE id=?2",params![w.get_business_hidden(),w.get_business_id()])?;if n!=1{return Err(invalid("Select an existing business"));}w.set_business_id(0);refresh(&w,&r.borrow())})();report(&w,result);});
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_topology_open(move |kind, id| {
        let Some(w) = weak.upgrade() else { return };
        if kind == "business" {
            w.set_business_hidden(false);
            w.set_business_id(0);
            w.invoke_tab_selected(6);
            w.invoke_business_select(id);
        } else if kind == "domain" || kind == "service" {
            w.invoke_tab_selected(if kind == "domain" { 5 } else { 4 });
            w.invoke_inventory_select(id);
            w.set_inventory_editor_requested(true);
        } else {
            let result = (|| {
                let servers = r.borrow().server_records()?;
                let s = servers
                    .iter()
                    .find(|s| s.id == i64::from(id))
                    .ok_or_else(|| invalid("Server no longer exists"))?;
                w.invoke_tab_selected(0);
                w.set_editing_id(id);
                w.set_editing_name(s.name.clone().into());
                w.set_editing_ip_address(s.ip_address.clone().into());
                w.set_editing_environment(s.environment.clone().into());
                w.set_editing_status(s.status.clone().into());
                w.set_editing_purpose(s.purpose.clone().into());
                w.set_editing_tags(s.tags.join(", ").into());
                w.set_editing_ports(s.ports.clone().into());
                w.set_server_editor_open(true);
                Ok(())
            })();
            report(&w, result);
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_topology_save_link(move || {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            let all = resources(&r.borrow())?;
            let source = all
                .get(w.get_topology_source() as usize)
                .ok_or_else(|| invalid("Choose a source resource"))?;
            let target = all
                .get(w.get_topology_target() as usize)
                .ok_or_else(|| invalid("Choose a target resource"))?;
            let relation = RELATIONS
                .get(w.get_topology_relation() as usize)
                .ok_or_else(|| invalid("Choose a relationship type"))?;
            r.borrow_mut().save_resource_link(
                i64::from(w.get_topology_link()),
                &source.kind,
                source.id,
                &target.kind,
                target.id,
                relation,
                w.get_topology_notes().as_str(),
            )?;
            w.set_topology_link(0);
            w.set_topology_notes("".into());
            refresh(&w, &r.borrow())
        })();
        report(&w, result);
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_topology_select_link(move |id| {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            let all = resources(&r.borrow())?;
            let links = r.borrow().resource_links()?;
            let l = links
                .iter()
                .find(|l| l.id == i64::from(id))
                .ok_or_else(|| invalid("Relationship no longer exists"))?;
            let a = all
                .iter()
                .position(|v| v.kind == l.source_type && v.id == l.source_id)
                .ok_or_else(|| invalid("Source is hidden"))?;
            let b = all
                .iter()
                .position(|v| v.kind == l.target_type && v.id == l.target_id)
                .ok_or_else(|| invalid("Target is hidden"))?;
            w.set_topology_source(a as i32);
            w.set_topology_target(b as i32);
            w.set_topology_relation(
                RELATIONS
                    .iter()
                    .position(|v| *v == l.relation_type)
                    .unwrap_or(9) as i32,
            );
            w.set_topology_notes(l.notes.clone().into());
            w.set_topology_link(id);
            Ok(())
        })();
        report(&w, result);
    });
    let weak = window.as_weak();
    let r = repo;
    window.on_topology_remove_link(move || {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            r.borrow_mut()
                .delete_resource_link(i64::from(w.get_topology_link()))?;
            w.set_topology_link(0);
            refresh(&w, &r.borrow())
        })();
        report(&w, result);
    });
}

/// Called only while constructing or explicitly resetting the in-memory demo.
pub fn reset_demo(repo: &mut Repository) -> Result<(), StorageError> {
    repo.connection.execute_batch("DELETE FROM resource_links; DELETE FROM operational_relations; DELETE FROM operational_inventory; DELETE FROM businesses;")?;
    repo.reset_demo_fixture()?;
    let business = repo.save_business(
        0,
        "Customer platform",
        "production, customer-facing",
        "Fictional business: portal, API, PostgreSQL and recovery",
    )?;
    let servers = repo.server_records()?;
    let host = |name: &str| {
        servers
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.id)
            .ok_or_else(|| invalid("Demo host missing"))
    };
    let edge = host("customer-edge-01")?;
    let api = host("api-prod-01")?;
    let db = host("postgres-primary")?;
    let backup = host("backup-01")?;
    let domain = repo.save_inventory(
        0,
        1,
        Details {
            name: "example.test".into(),
            business: "Customer platform".into(),
            registrar: "Example registrar".into(),
            registered: "2026-01-01".into(),
            expires: "2027-01-01".into(),
            ..Default::default()
        },
        &[],
        None,
    )?;
    let portal = repo.save_inventory(
        0,
        1,
        Details {
            name: "app.example.test".into(),
            parent: "example.test".into(),
            business: "Customer platform".into(),
            cdn: "Cloudflare (demo)".into(),
            ..Default::default()
        },
        &[edge],
        None,
    )?;
    let api_domain = repo.save_inventory(
        0,
        1,
        Details {
            name: "api.example.test".into(),
            parent: "example.test".into(),
            business: "Customer platform".into(),
            cdn: "Cloudflare (demo)".into(),
            ..Default::default()
        },
        &[api],
        None,
    )?;
    let mut saved = Vec::new();
    for (name, port, protocol, host, version) in [
        ("Customer HTTPS", "443", "HTTPS", edge, "1.26"),
        ("Orders API", "8080", "HTTP", api, "2.4"),
        ("PostgreSQL primary", "5432", "PostgreSQL", db, "16.4"),
        ("PostgreSQL replica", "5432", "PostgreSQL", backup, "16.4"),
    ] {
        saved.push(repo.save_inventory(
            0,
            0,
            Details {
                name: name.into(),
                ports: port.into(),
                protocol: protocol.into(),
                transport: "TCP".into(),
                version: version.into(),
                business: "Customer platform".into(),
                notes: "Fictional demo data".into(),
                ..Default::default()
            },
            &[host],
            None,
        )?);
    }
    for (kind, from, to, rel) in [
        ("domain", domain, portal, "contains"),
        ("domain", domain, api_domain, "contains"),
    ] {
        repo.save_resource_link(0, kind, from, kind, to, rel, "")?;
    }
    repo.save_resource_link(0, "business", business, "domain", domain, "owns", "")?;
    for (a, b, relation, note) in [
        (saved[0], saved[1], "depends_on", "Portal calls the API"),
        (
            saved[1],
            saved[2],
            "depends_on",
            "API reads and writes orders",
        ),
        (saved[2], saved[3], "replicates_to", "Streaming replication"),
        (
            saved[2],
            saved[3],
            "fails_over_to",
            "Promote replica during recovery",
        ),
        (
            saved[0],
            saved[1],
            "balances_to",
            "Example upstream routing",
        ),
    ] {
        repo.save_resource_link(0, "service", a, "service", b, relation, note)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;
    #[test]
    fn demo_reset_and_graph_edits_share_inventory() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        reset_demo(&mut repo)?;
        assert_eq!(repo.businesses()?.len(), 1);
        let entries = repo.inventory_entries()?;
        assert_eq!(entries.iter().filter(|e| e.kind == 1).count(), 3);
        assert_eq!(entries.iter().filter(|e| e.kind == 0).count(), 4);
        for relation in [
            "owns",
            "contains",
            "origin_on",
            "hosted_on",
            "depends_on",
            "replicates_to",
            "fails_over_to",
            "balances_to",
        ] {
            assert!(
                repo.resource_links()?
                    .iter()
                    .any(|l| l.relation_type == relation),
                "Missing {relation}"
            );
        }
        let service = entries
            .iter()
            .find(|e| e.details.name == "Orders API")
            .expect("API");
        let host = repo
            .server_records()?
            .into_iter()
            .find(|s| s.name == "backup-01")
            .expect("backup")
            .id;
        let link = repo.save_resource_link(
            0,
            "service",
            service.id,
            "server",
            host,
            "hosted_on",
            "additional host",
        )?;
        assert!(
            repo.inventory_entries()?
                .iter()
                .find(|e| e.id == service.id)
                .expect("service")
                .hosts
                .contains(&host)
        );
        let other_host = repo
            .server_records()?
            .into_iter()
            .find(|s| s.name == "customer-edge-01")
            .expect("edge")
            .id;
        assert_eq!(
            repo.save_resource_link(
                link,
                "service",
                service.id,
                "server",
                other_host,
                "hosted_on",
                "moved"
            )?,
            link
        );
        let moved = repo
            .inventory_entries()?
            .into_iter()
            .find(|e| e.id == service.id)
            .expect("service");
        assert!(!moved.hosts.contains(&host));
        assert!(moved.hosts.contains(&other_host));
        // An invalid edit must roll back both representations.
        assert!(
            repo.save_resource_link(
                link,
                "service",
                service.id,
                "server",
                i64::MAX,
                "hosted_on",
                ""
            )
            .is_err()
        );
        assert!(
            repo.inventory_entries()?
                .into_iter()
                .find(|e| e.id == service.id)
                .expect("service")
                .hosts
                .contains(&other_host)
        );
        repo.delete_resource_link(link)?;
        assert!(
            !repo
                .inventory_entries()?
                .iter()
                .find(|e| e.id == service.id)
                .expect("service")
                .hosts
                .contains(&host)
        );
        let dependency = repo
            .resource_links()?
            .into_iter()
            .find(|l| l.relation_type == "depends_on")
            .expect("dependency");
        repo.save_resource_link(
            dependency.id,
            "service",
            dependency.source_id,
            "service",
            dependency.target_id,
            "related_to",
            "changed",
        )?;
        assert!(
            repo.resource_links()?
                .iter()
                .any(|l| l.id == dependency.id && l.relation_type == "related_to")
        );
        repo.delete_resource_link(dependency.id)?;
        let count:i64=repo.connection.query_row("SELECT COUNT(*) FROM operational_relations WHERE source_id=?1 AND target_id=?2 AND kind=0",params![dependency.source_id,dependency.target_id],|r|r.get(0))?;
        assert_eq!(count, 0);
        let root = entries
            .iter()
            .find(|e| e.details.name == "example.test")
            .expect("root");
        let child = entries
            .iter()
            .find(|e| e.details.name == "app.example.test")
            .expect("child");
        assert!(
            repo.save_resource_link(0, "domain", child.id, "domain", root.id, "contains", "")
                .is_err()
        );
        assert!(
            repo.save_resource_link(0, "server", host, "domain", root.id, "owns", "")
                .is_err()
        );
        let mut bad_parent = root.details.clone();
        bad_parent.parent = child.details.name.clone();
        assert!(
            repo.save_inventory(root.id, 1, bad_parent, &root.hosts, None)
                .is_err()
        );
        let mut renamed = root.details.clone();
        renamed.name = "renamed.test".into();
        repo.save_inventory(root.id, 1, renamed, &root.hosts, None)?;
        assert_eq!(
            repo.inventory_entries()?
                .into_iter()
                .find(|e| e.id == child.id)
                .expect("child")
                .details
                .parent,
            "renamed.test"
        );
        let b = repo.businesses()?.remove(0);
        repo.save_business(b.id, "Renamed platform", "prod", "notes")?;
        assert!(
            repo.inventory_entries()?
                .iter()
                .all(|e| e.details.business == "Renamed platform")
        );
        reset_demo(&mut repo)?;
        assert_eq!(repo.inventory_entries()?.len(), 7);
        assert_eq!(repo.businesses()?.len(), 1);
        Ok(())
    }
    #[test]
    fn topology_bridge_renders_filters_and_opens_nodes() -> Result<(), StorageError> {
        i_slint_backend_testing::init_no_event_loop();
        let w = AppWindow::new().expect("window");
        let mut repository = Repository::in_memory()?;
        reset_demo(&mut repository)?;
        let repo = Rc::new(RefCell::new(repository));
        crate::operational::setup(&w, repo.clone());
        setup(&w, repo.clone());
        let weak = w.as_weak();
        w.on_tab_selected(move |tab| {
            if let Some(w) = weak.upgrade() {
                w.set_active_tab(tab);
            }
        });
        w.invoke_topology_refresh();
        assert_eq!(w.get_topology_nodes().row_count(), 16);
        assert!(w.get_topology_edges().row_count() > 10);
        let nodes = w.get_topology_nodes();
        let mut endpoints = BTreeSet::new();
        for edge in w.get_topology_edges().iter() {
            for point in [(edge.sx, edge.sy), (edge.tx, edge.ty)] {
                assert!(
                    endpoints.insert((point.0.to_bits(), point.1.to_bits())),
                    "arrows must not share the same node port: {}",
                    edge.label
                );
            }
            let points = [
                (edge.sx, edge.sy),
                (edge.mx, edge.sy),
                (edge.mx, edge.route_y),
                (edge.mx2, edge.route_y),
                (edge.mx2, edge.ty),
                (edge.tx, edge.ty),
            ];
            for segment in points.windows(2) {
                let [(x1, y1), (x2, y2)] = [segment[0], segment[1]];
                for node in nodes.iter() {
                    let crosses = if y1 == y2 {
                        y1 > node.py
                            && y1 < node.py + 66.0
                            && x1.min(x2) < node.px + 220.0
                            && x1.max(x2) > node.px
                    } else {
                        x1 > node.px
                            && x1 < node.px + 220.0
                            && y1.min(y2) < node.py + 66.0
                            && y1.max(y2) > node.py
                    };
                    assert!(!crosses, "{} crosses node {}", edge.label, node.label);
                }
            }
        }

        w.set_topology_filter(1);
        w.invoke_topology_refresh();
        assert!(w.get_topology_nodes().row_count() < 16);
        let api = repo
            .borrow()
            .inventory_entries()?
            .into_iter()
            .find(|e| e.details.name == "Orders API")
            .expect("API");
        use i_slint_backend_testing::ElementHandle;
        w.window().set_size(slint::PhysicalSize::new(1400, 950));
        w.set_active_tab(7);
        w.show().expect("show graph");
        assert_eq!(
            w.get_topology_node_choices().row_count(),
            w.get_topology_nodes().row_count()
        );
        ElementHandle::find_by_accessible_label(&w, "Select service: Orders API")
            .next()
            .expect("API node")
            .invoke_accessible_default_action();
        assert_eq!(w.get_active_tab(), 7, "selection must stay on map");
        assert!(!w.get_inventory_editor_requested());
        for label in ["Page down", "Page up", "Top"] {
            ElementHandle::find_by_accessible_label(&w, label)
                .next()
                .expect("navigation button")
                .invoke_accessible_default_action();
            assert_eq!(w.get_active_tab(), 7);
        }
        ElementHandle::find_by_accessible_label(&w, "Edit record")
            .next()
            .expect("explicit edit action")
            .invoke_accessible_default_action();
        assert_eq!(w.get_active_tab(), 4);
        assert_eq!(w.get_inventory_form().id, api.id as i32);
        assert!(w.get_inventory_editor_requested());
        w.invoke_business_select(0);
        w.set_business_name("Second business".into());
        w.invoke_business_save();
        assert_eq!(repo.borrow().businesses()?.len(), 2);
        w.invoke_business_hide();
        assert_eq!(repo.borrow().businesses()?.len(), 1);
        Ok(())
    }
}

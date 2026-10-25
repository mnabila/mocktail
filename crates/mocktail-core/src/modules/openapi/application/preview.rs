//! OpenAPI import preview: diffing a spec against existing mocks.

use crate::modules::collection::domain::Collection;
use crate::modules::folder::application::FolderRepository;
use crate::modules::mock::domain::Mock;
use crate::modules::openapi::domain::{MockDraft, SpecInfo, extract_spec_info, generate_drafts};
use crate::shared::error::AppResult;
use crate::shared::ids::{FolderId, MockId};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffAction {
    Added,
    Changed,
    Unchanged,
    Removed,
}

impl DiffAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffAction::Added => "added",
            DiffAction::Changed => "changed",
            DiffAction::Unchanged => "unchanged",
            DiffAction::Removed => "removed",
        }
    }
}

pub struct PreviewFolder {
    /// Canonical slash-separated folder path, e.g. `"Pets"` or `"Admin/Dangerous"`.
    pub path: String,
    /// Leaf folder name, e.g. `"Pets"` or `"Dangerous"`.
    pub name: String,
    /// Parent folder path if nested, e.g. `Some("Admin")`.
    pub parent_path: Option<String>,
    /// `Added` if the folder will be created; `Unchanged` if it already exists.
    pub action: DiffAction,
    pub existing_folder_id: Option<FolderId>,
}

pub struct PreviewItem {
    pub key: String,
    pub draft: Option<MockDraft>,
    pub action: DiffAction,
    pub existing_mock_id: Option<MockId>,
    /// Selected for import by default (2xx responses that change something).
    pub selected: bool,
}

pub struct ImportPreview {
    pub spec_info: SpecInfo,
    pub folders: Vec<PreviewFolder>,
    pub items: Vec<PreviewItem>,
}

impl ImportPreview {
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let mut added = 0;
        let mut changed = 0;
        let mut unchanged = 0;
        let mut removed = 0;
        for i in &self.items {
            match i.action {
                DiffAction::Added => added += 1,
                DiffAction::Changed => changed += 1,
                DiffAction::Unchanged => unchanged += 1,
                DiffAction::Removed => removed += 1,
            }
        }
        (added, changed, unchanged, removed)
    }

    pub fn folders_added_count(&self) -> usize {
        self.folders
            .iter()
            .filter(|f| f.action == DiffAction::Added)
            .count()
    }
}

pub struct ImportSelection {
    /// Keys to accept. `None` => default selection (2xx responses).
    pub accept: Option<HashSet<String>>,
    /// Delete existing mocks not present in the accepted set.
    pub delete_removed: bool,
    /// Auto-create folders from OpenAPI operation tags and place mocks in them.
    pub create_folders: bool,
}

impl Default for ImportSelection {
    fn default() -> Self {
        Self {
            accept: None,
            delete_removed: false,
            create_folders: true,
        }
    }
}

fn is_success(status: u16) -> bool {
    (200..=299).contains(&status)
}

/// Build a preview diff of a parsed spec against a collection's current mocks
/// and folders.
pub async fn preview_import(
    mock_repo: &dyn crate::modules::mock::application::MockRepository,
    folder_repo: &dyn FolderRepository,
    collection: &Collection,
    doc: &Value,
) -> AppResult<ImportPreview> {
    let existing = mock_repo
        .list_active(&collection.workspace_id, &collection.id)
        .await?;
    let existing_by_key: HashMap<String, &Mock> =
        existing.iter().map(|m| (mock_key(m), m)).collect();

    let existing_folders = folder_repo
        .list(&collection.workspace_id, &collection.id)
        .await?;
    let existing_folder_map: HashMap<(Option<String>, String), FolderId> = existing_folders
        .iter()
        .map(|f| {
            (
                (
                    f.parent_id.as_ref().map(|p| p.as_str().to_string()),
                    f.name.trim().to_lowercase(),
                ),
                f.id.clone(),
            )
        })
        .collect();

    let spec_info = extract_spec_info(doc);
    let drafts = generate_drafts(doc);
    let draft_keys: HashSet<String> = drafts.iter().map(MockDraft::key).collect();

    let mut folders = Vec::new();
    let mut seen_folder_paths: HashMap<String, Option<FolderId>> = HashMap::new();

    for draft in &drafts {
        let segments = draft.folder_segments();
        if segments.is_empty() {
            continue;
        }
        let mut parent_path: Option<String> = None;
        let mut parent_id: Option<String> = None;
        let mut parent_exists = true;

        for seg in segments {
            let current_path = match &parent_path {
                Some(p) => format!("{p}/{seg}"),
                None => seg.clone(),
            };
            let path_key = current_path.to_lowercase();
            if let Some(existing_id) = seen_folder_paths.get(&path_key) {
                parent_exists = existing_id.is_some();
                parent_id = existing_id.as_ref().map(|id| id.as_str().to_string());
                parent_path = Some(current_path);
                continue;
            }

            let matched_id = if parent_exists {
                existing_folder_map
                    .get(&(parent_id.clone(), seg.to_lowercase()))
                    .cloned()
            } else {
                None
            };

            let action = if matched_id.is_some() {
                DiffAction::Unchanged
            } else {
                DiffAction::Added
            };
            folders.push(PreviewFolder {
                path: current_path.clone(),
                name: seg,
                parent_path: parent_path.clone(),
                action,
                existing_folder_id: matched_id.clone(),
            });
            seen_folder_paths.insert(path_key, matched_id.clone());
            parent_exists = matched_id.is_some();
            parent_id = matched_id.map(|id| id.as_str().to_string());
            parent_path = Some(current_path);
        }
    }

    let mut items = Vec::new();

    for draft in drafts {
        let key = draft.key();
        let (action, existing_id) = match existing_by_key.get(&key) {
            None => (DiffAction::Added, None),
            Some(m) => {
                let same = m.response.body == draft.body && m.response.status == draft.status;
                let action = if same {
                    DiffAction::Unchanged
                } else {
                    DiffAction::Changed
                };
                (action, Some(m.id.clone()))
            }
        };
        let selected = is_success(draft.status) && action != DiffAction::Unchanged;
        items.push(PreviewItem {
            key,
            action,
            existing_mock_id: existing_id,
            selected,
            draft: Some(draft),
        });
    }

    // Existing mocks absent from the spec are candidates for removal.
    for m in &existing {
        let key = mock_key(m);
        if !draft_keys.contains(&key) {
            items.push(PreviewItem {
                key,
                draft: None,
                action: DiffAction::Removed,
                existing_mock_id: Some(m.id.clone()),
                selected: false,
            });
        }
    }

    Ok(ImportPreview {
        spec_info,
        folders,
        items,
    })
}

/// The stable identity key of a stored mock, matching `MockDraft::key`.
pub(super) fn mock_key(m: &Mock) -> String {
    format!(
        "{} {} {}",
        m.method.as_str(),
        m.path_template,
        m.response.status
    )
}

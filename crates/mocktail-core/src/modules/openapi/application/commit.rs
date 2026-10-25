//! OpenAPI import commitment and export.

use super::preview::{ImportSelection, mock_key};
use crate::modules::collection::domain::Collection;
use crate::modules::folder::application::FolderRepository;
use crate::modules::folder::domain::Folder;
use crate::modules::mock::application::{MockRepository, save_mock};
use crate::modules::mock::domain::{Mock, Priority, ResponseSpec};
use crate::modules::openapi::domain::{export_document_with_folders, generate_drafts};
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{FolderId, MockId};
use crate::shared::ports::{Clock, IdGenerator};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ImportResult {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub folders_created: usize,
}

/// Commit an import. Accepted drafts are upserted, reusing an existing mock's
/// id when its key matches (round-trip update in place).
/// When `selection.create_folders` is true, operation `tags` automatically
/// create (or reuse) folders in the collection and assign `folder_id` on each mock.
pub async fn commit_import(
    mock_repo: &dyn MockRepository,
    folder_repo: &dyn FolderRepository,
    ids: &dyn IdGenerator,
    clock: &dyn Clock,
    collection: &Collection,
    doc: &Value,
    selection: ImportSelection,
) -> AppResult<ImportResult> {
    let existing = mock_repo
        .list_active(&collection.workspace_id, &collection.id)
        .await?;
    let existing_by_key: HashMap<String, Mock> =
        existing.iter().map(|m| (mock_key(m), m.clone())).collect();

    let existing_folders = folder_repo
        .list(&collection.workspace_id, &collection.id)
        .await?;
    let mut folder_map: HashMap<(Option<String>, String), FolderId> = existing_folders
        .into_iter()
        .map(|f| {
            (
                (
                    f.parent_id.as_ref().map(|p| p.as_str().to_string()),
                    f.name.trim().to_lowercase(),
                ),
                f.id,
            )
        })
        .collect();

    let drafts = generate_drafts(doc);
    let now = clock.now_millis();
    let mut result = ImportResult::default();
    let mut accepted_keys: HashSet<String> = HashSet::new();

    for draft in &drafts {
        let key = draft.key();
        let accept = match &selection.accept {
            Some(set) => set.contains(&key),
            None => {
                // Default selection: 2xx responses.
                (200..=299).contains(&draft.status)
            }
        };
        if !accept {
            continue;
        }
        accepted_keys.insert(key.clone());

        let (id, created_at, existing_folder_id, is_update) = match existing_by_key.get(&key) {
            Some(m) => (m.id.clone(), m.created_at, m.folder_id.clone(), true),
            None => (MockId::new(ids.new_object_id()), now, None, false),
        };

        let folder_id = if selection.create_folders {
            let segments = draft.folder_segments();
            if !segments.is_empty() {
                let mut parent_id: Option<FolderId> = None;
                for seg in segments {
                    let lookup_key = (
                        parent_id.as_ref().map(|p| p.as_str().to_string()),
                        seg.to_lowercase(),
                    );
                    if let Some(found) = folder_map.get(&lookup_key) {
                        parent_id = Some(found.clone());
                    } else {
                        let new_folder_id = FolderId::new(ids.new_object_id());
                        let folder = Folder {
                            id: new_folder_id.clone(),
                            workspace_id: collection.workspace_id.clone(),
                            collection_id: collection.id.clone(),
                            name: seg,
                            parent_id: parent_id.clone(),
                            created_at: now,
                        };
                        folder_repo.insert(&folder).await?;
                        folder_map.insert(lookup_key, new_folder_id.clone());
                        result.folders_created += 1;
                        parent_id = Some(new_folder_id);
                    }
                }
                parent_id
            } else {
                existing_folder_id
            }
        } else {
            existing_folder_id
        };

        let mut headers = Vec::new();
        if let Some(ct) = &draft.content_type
            && !draft.body.is_empty()
        {
            headers.push(("content-type".to_string(), ct.clone()));
        }

        let mock = Mock {
            id,
            workspace_id: collection.workspace_id.clone(),
            collection_id: collection.id.clone(),
            folder_id,
            method: draft.method,
            path_template: draft.path_template.clone(),
            matchers: Vec::new(),
            response: ResponseSpec {
                status: draft.status,
                headers,
                body: draft.body.clone(),
            },
            priority: Priority(0),
            delay_ms: 0,
            fault: None,
            rate_limit: None,
            created_at,
            updated_at: now,
        };
        save_mock(mock_repo, &mock).await?;
        if is_update {
            result.updated += 1;
        } else {
            result.created += 1;
        }
    }

    if selection.delete_removed {
        for m in &existing {
            if !accepted_keys.contains(&mock_key(m)) {
                mock_repo.delete(&m.id).await?;
                result.deleted += 1;
            }
        }
    }

    Ok(result)
}

/// Serialize an exported collection to bytes in the requested format.
pub enum ExportFormat {
    Json,
    Yaml,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "yaml" | "yml" => ExportFormat::Yaml,
            _ => ExportFormat::Json,
        }
    }
    pub fn content_type(&self) -> &'static str {
        match self {
            ExportFormat::Json => "application/json",
            ExportFormat::Yaml => "application/yaml",
        }
    }
}

/// Export a collection's mocks to an OpenAPI document (loads via the port).
pub async fn export_collection(
    mock_repo: &dyn MockRepository,
    folder_repo: &dyn FolderRepository,
    collection: &Collection,
    format: ExportFormat,
) -> AppResult<(Vec<u8>, &'static str)> {
    let mocks = mock_repo
        .list_active(&collection.workspace_id, &collection.id)
        .await?;
    let folders = folder_repo
        .list(&collection.workspace_id, &collection.id)
        .await?;
    let doc =
        export_document_with_folders(&collection.name, &collection.base_path, &mocks, &folders);
    let bytes = match format {
        ExportFormat::Json => {
            serde_json::to_vec_pretty(&doc).map_err(|e| AppError::Internal(e.to_string()))?
        }
        ExportFormat::Yaml => serde_yaml::to_string(&doc)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .into_bytes(),
    };
    Ok((bytes, format.content_type()))
}

//! OpenAPI collection export.

use crate::modules::collection::domain::Collection;
use crate::modules::folder::application::FolderRepository;
use crate::modules::mock::application::MockRepository;
use crate::modules::openapi::application::commit::ExportFormat;
use crate::modules::openapi::domain::export_document_with_folders;
use crate::shared::error::{AppError, AppResult};

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

//! Catalog initialization helpers — Memory and REST catalog backends.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::types::observability::IcebergConfig;
use iceberg::io::FileIO;
use iceberg::memory::{MEMORY_CATALOG_WAREHOUSE, MemoryCatalogBuilder};
use iceberg::{Catalog, CatalogBuilder, NamespaceIdent};
use iceberg_catalog_rest::{REST_CATALOG_PROP_URI, RestCatalogBuilder};

/// A thread-safe reference to an Iceberg catalog.
pub type CatalogRef = Arc<dyn Catalog>;

/// Build a catalog from the given [`IcebergConfig`].
///
/// | `catalog_uri` | Backend used |
/// |---|---|
/// | `None` | In-memory catalog with filesystem `FileIO` |
/// | `Some(uri)` | REST catalog pointing at `uri` |
pub async fn build_catalog(config: &IcebergConfig) -> Result<CatalogRef> {
    if let Some(uri) = &config.catalog_uri {
        build_rest_catalog(uri).await
    } else {
        build_memory_catalog(config).await
    }
}

async fn build_memory_catalog(config: &IcebergConfig) -> Result<CatalogRef> {
    let warehouse = warehouse_path(config);
    let props = HashMap::from([(MEMORY_CATALOG_WAREHOUSE.to_string(), warehouse)]);
    let catalog = MemoryCatalogBuilder::default()
        .load("memory", props)
        .await
        .context("failed to create in-memory Iceberg catalog")?;
    Ok(Arc::new(catalog))
}

async fn build_rest_catalog(uri: &str) -> Result<CatalogRef> {
    let props = HashMap::from([(REST_CATALOG_PROP_URI.to_string(), uri.to_string())]);
    let catalog = RestCatalogBuilder::default()
        .load("rest", props)
        .await
        .context("failed to create REST Iceberg catalog")?;
    Ok(Arc::new(catalog))
}

/// Build a [`FileIO`] instance for writing Parquet data files.
///
/// The `_config` parameter is reserved for future backend selection (S3, GCS,
/// Azure ADLS) once those storage factories are wired up.
pub fn build_file_io(_config: &IcebergConfig) -> FileIO {
    FileIO::new_with_fs()
}

/// Ensure the target namespace exists in the catalog, creating it if necessary.
pub async fn ensure_namespace(catalog: &dyn Catalog, namespace: &str) -> Result<NamespaceIdent> {
    let ns = NamespaceIdent::new(namespace.to_string());
    if !catalog
        .namespace_exists(&ns)
        .await
        .context("failed to check namespace existence")?
    {
        catalog
            .create_namespace(&ns, HashMap::new())
            .await
            .context("failed to create namespace")?;
    }
    Ok(ns)
}

fn warehouse_path(config: &IcebergConfig) -> String {
    config.warehouse.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| {
                h.join(".assistant")
                    .join("iceberg")
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|d| {
                        d.join(".assistant")
                            .join("iceberg")
                            .to_string_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|_| "./.assistant/iceberg".to_string())
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::observability::PartitionGranularity;

    fn test_config(warehouse: Option<&str>) -> IcebergConfig {
        IcebergConfig {
            warehouse: warehouse.map(String::from),
            namespace: "test".to_string(),
            partition: PartitionGranularity::None,
            catalog_uri: None,
        }
    }

    #[test]
    fn warehouse_path_uses_explicit_value() {
        let cfg = test_config(Some("/custom/wh"));
        assert_eq!(warehouse_path(&cfg), "/custom/wh");
    }

    #[test]
    fn warehouse_path_falls_back_to_assistant_iceberg() {
        let cfg = test_config(None);
        // Should resolve to ~/.assistant/iceberg or ./.assistant/iceberg.
        let p = warehouse_path(&cfg);
        assert!(p.ends_with(".assistant/iceberg"), "got: {p}");
    }

    #[test]
    fn build_file_io_returns_filesystem_backend() {
        let cfg = test_config(Some("/tmp/wh"));
        // Smoke test: the function returns successfully and produces a FileIO.
        let _io = build_file_io(&cfg);
    }

    #[tokio::test]
    async fn build_catalog_creates_memory_catalog_when_no_uri() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_config(Some(tmp.path().to_str().unwrap()));
        let catalog = build_catalog(&cfg).await.unwrap();

        let ns = ensure_namespace(catalog.as_ref(), "ns_test").await.unwrap();
        assert_eq!(ns.to_string(), "ns_test");
    }

    #[tokio::test]
    async fn ensure_namespace_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = test_config(Some(tmp.path().to_str().unwrap()));
        let catalog = build_catalog(&cfg).await.unwrap();

        let first = ensure_namespace(catalog.as_ref(), "ns_dup").await.unwrap();
        let second = ensure_namespace(catalog.as_ref(), "ns_dup").await.unwrap();
        assert_eq!(first.to_string(), second.to_string());
    }
}

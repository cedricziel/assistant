//! Catalog initialization helpers — Memory and REST catalog backends.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::IcebergConfig;
use iceberg::io::FileIO;
use iceberg::memory::{MemoryCatalogBuilder, MEMORY_CATALOG_WAREHOUSE};
use iceberg::{Catalog, CatalogBuilder, NamespaceIdent};
use iceberg_catalog_rest::{RestCatalogBuilder, REST_CATALOG_PROP_URI};

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

/// Build the filesystem `FileIO` used to write Parquet data files.
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
            .unwrap_or_else(|| "/tmp/.assistant/iceberg".to_string())
    })
}

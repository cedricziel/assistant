//! Converts [`PartitionGranularity`] config values to Iceberg [`UnboundPartitionSpec`]s.

use anyhow::Result;
use assistant_core::PartitionGranularity;
use iceberg::spec::{Transform, UnboundPartitionSpec};

/// Build an [`UnboundPartitionSpec`] that partitions a table by the given time
/// column field ID, using the requested granularity.
///
/// Returns `None` when `granularity` is [`PartitionGranularity::None`].
pub fn partition_spec(
    granularity: &PartitionGranularity,
    time_field_id: i32,
    target_name: &str,
) -> Result<Option<UnboundPartitionSpec>> {
    let transform = match granularity {
        PartitionGranularity::None => return Ok(None),
        PartitionGranularity::Year => Transform::Year,
        PartitionGranularity::Month => Transform::Month,
        PartitionGranularity::Day => Transform::Day,
        PartitionGranularity::Hour => Transform::Hour,
    };

    let spec = UnboundPartitionSpec::builder()
        .add_partition_field(time_field_id, target_name, transform)?
        .build();

    Ok(Some(spec))
}

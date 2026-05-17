//! Converts [`PartitionGranularity`] config values to Iceberg [`UnboundPartitionSpec`]s,
//! and computes representative [`PartitionKey`]s for batch writes.

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use chrono::{Datelike, TimeZone, Utc};
use iceberg::spec::{Literal, PartitionKey, SchemaRef, Struct, Transform, UnboundPartitionSpec};
use iceberg::table::Table;

use assistant_core::types::observability::PartitionGranularity;

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

/// Compute a single representative [`PartitionKey`] for a batch of OTel
/// records whose timestamps all fall within the same partition bucket.
///
/// OTel export batches are typically a few seconds wide, so treating the
/// first record's timestamp as representative for the whole batch is correct
/// in practice (records from midnight are the only exception).
///
/// Returns `None` when:
/// - `granularity` is [`PartitionGranularity::None`], or
/// - the table's default partition spec is unpartitioned.
pub fn batch_partition_key(
    table: &Table,
    representative_micros: i64,
    granularity: &PartitionGranularity,
) -> Option<PartitionKey> {
    if matches!(granularity, PartitionGranularity::None) {
        return None;
    }

    let spec = (**table.metadata().default_partition_spec()).clone();
    if spec.is_unpartitioned() {
        return None;
    }

    let schema: SchemaRef = table.metadata().current_schema().clone();
    let partition_value = granularity_to_partition_int(representative_micros, granularity);
    let data = Struct::from_iter(vec![Some(Literal::int(partition_value))]);

    Some(PartitionKey::new(spec, schema, data))
}

/// Convert a microsecond-since-epoch timestamp to the integer partition value
/// expected by Iceberg's time-based transforms.
fn granularity_to_partition_int(micros: i64, granularity: &PartitionGranularity) -> i32 {
    match granularity {
        PartitionGranularity::None => 0,
        PartitionGranularity::Hour => (micros / 3_600_000_000_i64) as i32,
        PartitionGranularity::Day => (micros / 86_400_000_000_i64) as i32,
        PartitionGranularity::Month => {
            let secs = micros.div_euclid(1_000_000);
            let nsecs = (micros.rem_euclid(1_000_000) * 1000) as u32;
            let dt = Utc
                .timestamp_opt(secs, nsecs)
                .single()
                .unwrap_or_else(|| SystemClock.now());
            (dt.year() - 1970) * 12 + (dt.month() as i32 - 1)
        }
        PartitionGranularity::Year => {
            let secs = micros.div_euclid(1_000_000);
            let nsecs = (micros.rem_euclid(1_000_000) * 1000) as u32;
            let dt = Utc
                .timestamp_opt(secs, nsecs)
                .single()
                .unwrap_or_else(|| SystemClock.now());
            dt.year() - 1970
        }
    }
}

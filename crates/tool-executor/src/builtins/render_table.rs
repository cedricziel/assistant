//! Builds `data_table` render payloads within the render budget (design D4):
//! at most [`MAX_ROWS`] rows, every string cell at most [`MAX_CELL_CHARS`]
//! characters, cut on a character boundary.

use serde_json::{Map, Value};

use assistant_core::tool_render::{DataTableData, TableColumn, ToolRenderData};

/// Rows kept in a rendered table.
pub(crate) const MAX_ROWS: usize = 200;
/// Characters kept in one string cell, including the trailing `…`.
pub(crate) const MAX_CELL_CHARS: usize = 256;

/// A column as `(key, label)`.
pub(crate) type Column = (&'static str, &'static str);

/// Build a `data_table` from `rows` (one JSON object per row, keyed by column
/// key). `truncated` is set when rows were dropped or a cell was cut;
/// `total_rows` is the row count before any cut.
pub(crate) fn data_table(columns: &[Column], rows: Vec<Map<String, Value>>) -> ToolRenderData {
    let total_rows = rows.len() as u64;
    let mut truncated = rows.len() > MAX_ROWS;
    let rows = rows
        .into_iter()
        .take(MAX_ROWS)
        .map(|row| {
            row.into_iter()
                .map(|(key, value)| (key, cap_cell(value, &mut truncated)))
                .collect()
        })
        .collect();
    ToolRenderData::DataTable(DataTableData {
        columns: columns
            .iter()
            .map(|(key, label)| TableColumn {
                key: (*key).to_string(),
                label: (*label).to_string(),
                column_type: None,
            })
            .collect(),
        rows,
        truncated,
        total_rows,
    })
}

fn cap_cell(value: Value, truncated: &mut bool) -> Value {
    match value {
        Value::String(s) if s.chars().count() > MAX_CELL_CHARS => {
            *truncated = true;
            let mut cut: String = s.chars().take(MAX_CELL_CHARS - 1).collect();
            cut.push('…');
            Value::String(cut)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn row(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn keeps_small_tables_whole() {
        let ToolRenderData::DataTable(t) = data_table(&[("a", "A")], vec![row(json!({"a": "x"}))])
        else {
            panic!("expected data_table");
        };
        assert_eq!(t.total_rows, 1);
        assert!(!t.truncated);
        assert_eq!(t.rows[0]["a"], "x");
    }

    #[test]
    fn non_string_cells_pass_through() {
        let ToolRenderData::DataTable(t) =
            data_table(&[("n", "N")], vec![row(json!({"n": 7, "b": true}))])
        else {
            panic!("expected data_table");
        };
        assert_eq!(t.rows[0]["n"], 7);
        assert!(!t.truncated);
    }

    #[test]
    fn cuts_rows_and_cells() {
        let rows = (0..201)
            .map(|_| row(json!({"a": "é".repeat(300)})))
            .collect();
        let ToolRenderData::DataTable(t) = data_table(&[("a", "A")], rows) else {
            panic!("expected data_table");
        };
        assert_eq!(t.rows.len(), MAX_ROWS);
        assert_eq!(t.total_rows, 201);
        assert!(t.truncated);
        let cell = t.rows[0]["a"].as_str().unwrap_or_default();
        assert_eq!(cell.chars().count(), MAX_CELL_CHARS);
        assert!(cell.ends_with('…'));
    }
}

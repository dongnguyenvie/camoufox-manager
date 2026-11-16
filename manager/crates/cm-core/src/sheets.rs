//! Excel/CSV bulk import/export (phase 11).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SheetFormat {
    Xlsx,
    Csv,
}

/// One changed field found by a dry run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Dry-run diff for one sheet row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowDiff {
    /// 1-based row in the sheet.
    pub row: usize,
    pub profile_id: Option<String>,
    pub changes: Vec<FieldChange>,
    /// Per-field validation errors; a row with errors is not applied.
    pub errors: BTreeMap<String, String>,
}

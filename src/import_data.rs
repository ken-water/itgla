use std::{fs, path::Path};

use calamine::{Reader, open_workbook_auto};
use thiserror::Error;

const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_ROWS: usize = 10_000;
const MAX_COLUMNS: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabularData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Cannot read the selected file")]
    Io(#[from] std::io::Error),
    #[error("Cannot parse the delimited file")]
    Csv(#[from] csv::Error),
    #[error("Cannot parse the spreadsheet")]
    Spreadsheet(#[from] calamine::Error),
    #[error("Use an Excel, OpenDocument, CSV, or TSV file")]
    UnsupportedFormat,
    #[error("The import file must be 20 MiB or smaller")]
    FileTooLarge,
    #[error("The import file needs a header row and at least one data row")]
    Empty,
    #[error("The import can contain at most 100 columns")]
    TooManyColumns,
    #[error("The import can contain at most 10,000 rows")]
    TooManyRows,
}

pub fn read_tabular(path: &Path) -> Result<TabularData, ImportError> {
    if fs::metadata(path)?.len() > MAX_FILE_BYTES {
        return Err(ImportError::FileTooLarge);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(ImportError::UnsupportedFormat)?;
    match extension.as_str() {
        "csv" => read_delimited(path, b','),
        "tsv" => read_delimited(path, b'\t'),
        "xlsx" | "xls" | "xlsb" | "ods" => read_spreadsheet(path),
        _ => Err(ImportError::UnsupportedFormat),
    }
}

fn read_delimited(path: &Path, delimiter: u8) -> Result<TabularData, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_path(path)?;
    let headers = normalize_headers(reader.headers()?.iter());
    validate_column_count(headers.len())?;
    let mut rows = Vec::new();
    for result in reader.records() {
        if rows.len() == MAX_ROWS {
            return Err(ImportError::TooManyRows);
        }
        let record = result?;
        rows.push(pad_row(
            record.iter().map(str::to_owned).collect(),
            headers.len(),
        ));
    }
    finish(headers, rows)
}

fn read_spreadsheet(path: &Path) -> Result<TabularData, ImportError> {
    let mut workbook = open_workbook_auto(path)?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or(ImportError::Empty)?;
    let range = workbook.worksheet_range(&sheet_name)?;
    let mut source_rows = range.rows();
    let header_row = source_rows.next().ok_or(ImportError::Empty)?;
    let headers = normalize_headers(header_row.iter().map(ToString::to_string));
    validate_column_count(headers.len())?;
    let mut rows = Vec::new();
    for row in source_rows {
        if rows.len() == MAX_ROWS {
            return Err(ImportError::TooManyRows);
        }
        let values = pad_row(row.iter().map(ToString::to_string).collect(), headers.len());
        if values.iter().any(|value| !value.trim().is_empty()) {
            rows.push(values);
        }
    }
    finish(headers, rows)
}

fn normalize_headers<I, S>(headers: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut normalized = Vec::new();
    for (index, raw) in headers.into_iter().enumerate() {
        let base = raw.as_ref().trim();
        let base = if base.is_empty() {
            format!("Column {}", index + 1)
        } else {
            base.to_owned()
        };
        let mut candidate = base.clone();
        let mut suffix = 2;
        while normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&candidate))
        {
            candidate = format!("{base} ({suffix})");
            suffix += 1;
        }
        normalized.push(candidate);
    }
    normalized
}

fn pad_row(mut row: Vec<String>, width: usize) -> Vec<String> {
    row.truncate(width);
    row.resize(width, String::new());
    row
}

fn validate_column_count(columns: usize) -> Result<(), ImportError> {
    if columns == 0 {
        return Err(ImportError::Empty);
    }
    if columns > MAX_COLUMNS {
        return Err(ImportError::TooManyColumns);
    }
    Ok(())
}

fn finish(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<TabularData, ImportError> {
    if rows.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(TabularData { headers, rows })
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temporary_path(extension: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        env::temp_dir().join(format!("itgla-import-{nonce}.{extension}"))
    }

    #[test]
    fn reads_csv_and_normalizes_duplicate_headers() -> Result<(), ImportError> {
        let path = temporary_path("csv");
        fs::write(&path, "IP,Ports,IP,\n203.0.113.10,443,primary,owner\n")?;
        let data = read_tabular(&path)?;
        assert_eq!(data.headers, ["IP", "Ports", "IP (2)", "Column 4"]);
        assert_eq!(data.rows[0][1], "443");
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn reads_tab_separated_rows() -> Result<(), ImportError> {
        let path = temporary_path("tsv");
        fs::write(&path, "Tags\tIP\tPorts\napi\t2001:db8::1\t22,443\n")?;
        let data = read_tabular(&path)?;
        assert_eq!(data.rows[0], ["api", "2001:db8::1", "22,443"]);
        let _ = fs::remove_file(path);
        Ok(())
    }
}

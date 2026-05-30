use std::fmt;

/// Inferred column types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    String,
    Empty,
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColumnType::Integer => write!(f, "integer"),
            ColumnType::Float => write!(f, "float"),
            ColumnType::Boolean => write!(f, "boolean"),
            ColumnType::Date => write!(f, "date"),
            ColumnType::DateTime => write!(f, "datetime"),
            ColumnType::String => write!(f, "string"),
            ColumnType::Empty => write!(f, "empty"),
        }
    }
}

/// Infer the type of a single cell value.
pub fn infer_cell_type(value: &str) -> ColumnType {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return ColumnType::Empty;
    }

    // Boolean
    let lower = trimmed.to_lowercase();
    if lower == "true" || lower == "false" || lower == "yes" || lower == "no" {
        return ColumnType::Boolean;
    }

    // Integer
    if trimmed.parse::<i64>().is_ok() {
        return ColumnType::Integer;
    }

    // Float
    if trimmed.parse::<f64>().is_ok() {
        return ColumnType::Float;
    }

    // Date patterns: YYYY-MM-DD
    if trimmed.len() == 10
        && trimmed.as_bytes()[4] == b'-'
        && trimmed.as_bytes()[7] == b'-'
        && trimmed[0..4].parse::<u16>().is_ok()
        && trimmed[5..7].parse::<u8>().is_ok()
        && trimmed[8..10].parse::<u8>().is_ok()
    {
        return ColumnType::Date;
    }

    // DateTime patterns: YYYY-MM-DD HH:MM:SS or YYYY-MM-DDTHH:MM:SS
    if trimmed.len() >= 19 {
        let date_part = &trimmed[..10];
        let sep = trimmed.as_bytes()[10];
        if (sep == b'T' || sep == b' ')
            && date_part.as_bytes()[4] == b'-'
            && date_part.as_bytes()[7] == b'-'
        {
            return ColumnType::DateTime;
        }
    }

    ColumnType::String
}

/// Merge two inferred types into the broader compatible type.
pub fn merge_types(a: &ColumnType, b: &ColumnType) -> ColumnType {
    if a == b {
        return a.clone();
    }
    if *a == ColumnType::Empty {
        return b.clone();
    }
    if *b == ColumnType::Empty {
        return a.clone();
    }
    // Integer + Float => Float
    if (*a == ColumnType::Integer && *b == ColumnType::Float)
        || (*a == ColumnType::Float && *b == ColumnType::Integer)
    {
        return ColumnType::Float;
    }
    // Date + DateTime => DateTime
    if (*a == ColumnType::Date && *b == ColumnType::DateTime)
        || (*a == ColumnType::DateTime && *b == ColumnType::Date)
    {
        return ColumnType::DateTime;
    }
    // Everything else => String
    ColumnType::String
}

/// Detect the delimiter of a file from its extension or content.
pub fn detect_delimiter(path: &str) -> u8 {
    if path.ends_with(".tsv") {
        b'\t'
    } else if path.ends_with(".csv") {
        b','
    } else {
        // Default to comma
        b','
    }
}

/// Detect format from file extension.
#[derive(Debug, Clone, PartialEq)]
pub enum FileFormat {
    Csv,
    Tsv,
    Json,
    Jsonl,
}

impl FileFormat {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let lower = path.to_lowercase();
        if lower.ends_with(".csv") {
            Ok(FileFormat::Csv)
        } else if lower.ends_with(".tsv") {
            Ok(FileFormat::Tsv)
        } else if lower.ends_with(".jsonl") || lower.ends_with(".ndjson") {
            Ok(FileFormat::Jsonl)
        } else if lower.ends_with(".json") {
            Ok(FileFormat::Json)
        } else {
            anyhow::bail!("Unknown file format for: {path}. Supported: .csv, .tsv, .json, .jsonl")
        }
    }
}

/// Check if a value looks null/empty.
pub fn is_null_value(s: &str) -> bool {
    let t = s.trim();
    t.is_empty()
        || t.eq_ignore_ascii_case("null")
        || t.eq_ignore_ascii_case("na")
        || t.eq_ignore_ascii_case("n/a")
        || t == "."
        || t == "-"
        || t.eq_ignore_ascii_case("none")
        || t.eq_ignore_ascii_case("nan")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_scalar_types() {
        assert_eq!(infer_cell_type("42"), ColumnType::Integer);
        assert_eq!(infer_cell_type("-7"), ColumnType::Integer);
        assert_eq!(infer_cell_type("3.14"), ColumnType::Float);
        assert_eq!(infer_cell_type("true"), ColumnType::Boolean);
        assert_eq!(infer_cell_type("No"), ColumnType::Boolean);
        assert_eq!(infer_cell_type(""), ColumnType::Empty);
        assert_eq!(infer_cell_type("hello"), ColumnType::String);
    }

    #[test]
    fn infers_date_and_datetime() {
        assert_eq!(infer_cell_type("2024-01-15"), ColumnType::Date);
        assert_eq!(infer_cell_type("2024-01-15T08:30:00"), ColumnType::DateTime);
        assert_eq!(infer_cell_type("2024-01-15 08:30:00"), ColumnType::DateTime);
        // Inference is structural (separators + numeric components), not a
        // calendar check: a value not matching the YYYY-MM-DD shape is a String.
        assert_eq!(infer_cell_type("2024/01/15"), ColumnType::String);
        assert_eq!(infer_cell_type("not-a-date"), ColumnType::String);
    }

    #[test]
    fn merge_widens_to_compatible_type() {
        assert_eq!(
            merge_types(&ColumnType::Integer, &ColumnType::Float),
            ColumnType::Float
        );
        assert_eq!(
            merge_types(&ColumnType::Date, &ColumnType::DateTime),
            ColumnType::DateTime
        );
        // Empty is absorbed by the other type.
        assert_eq!(
            merge_types(&ColumnType::Empty, &ColumnType::Integer),
            ColumnType::Integer
        );
        // Identical types are preserved.
        assert_eq!(
            merge_types(&ColumnType::Boolean, &ColumnType::Boolean),
            ColumnType::Boolean
        );
        // Incompatible types collapse to String.
        assert_eq!(
            merge_types(&ColumnType::Integer, &ColumnType::Boolean),
            ColumnType::String
        );
    }

    #[test]
    fn detects_null_like_values() {
        for v in ["", "  ", "null", "NA", "n/a", ".", "-", "None", "NaN"] {
            assert!(is_null_value(v), "expected {v:?} to be null-like");
        }
        for v in ["0", "false", "x", "value"] {
            assert!(!is_null_value(v), "expected {v:?} not to be null-like");
        }
    }

    #[test]
    fn detects_delimiter_from_extension() {
        assert_eq!(detect_delimiter("data.tsv"), b'\t');
        assert_eq!(detect_delimiter("data.csv"), b',');
        // Unknown extension defaults to comma.
        assert_eq!(detect_delimiter("data.txt"), b',');
    }

    #[test]
    fn file_format_from_path() {
        assert_eq!(FileFormat::from_path("a.CSV").unwrap(), FileFormat::Csv);
        assert_eq!(FileFormat::from_path("a.tsv").unwrap(), FileFormat::Tsv);
        assert_eq!(FileFormat::from_path("a.json").unwrap(), FileFormat::Json);
        assert_eq!(FileFormat::from_path("a.jsonl").unwrap(), FileFormat::Jsonl);
        assert_eq!(
            FileFormat::from_path("a.ndjson").unwrap(),
            FileFormat::Jsonl
        );
        assert!(FileFormat::from_path("a.parquet").is_err());
    }
}

//! The CSV result format shared by the harness and the Go and Java controls.
//!
//! One row per measured metric ("long" or tidy format), so that every benchmark,
//! strategy and control writes the same columns and a report can be produced with
//! a single pivot. The column list is [`HEADER`]; the controls in `controls/` copy
//! it verbatim. A file is opened in append mode and gets the header only when it
//! is empty, so several runs can share one file.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Schema version; bump when a column is added, removed or changes meaning.
pub const SCHEMA_VERSION: u32 = 1;

/// The header line (without the trailing newline).
pub const HEADER: &str = "schema,run_id,timestamp_utc,machine_id,implementation,benchmark,strategy,reservation_bytes,stack_bytes,threads,tasks,params,metric,value,unit,status,note";

/// Outcome of the measurement a row describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Measured successfully.
    Ok,
    /// The configuration failed (for example a stack could not be allocated); the
    /// note says why. A failure is a result, not an error of the harness.
    Failed,
    /// Not run here (for example io_uring unavailable); the note says why.
    Skipped,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Failed => "failed",
            Status::Skipped => "skipped",
        }
    }
}

/// One CSV row. Fields that do not apply are left empty.
#[derive(Clone, Debug)]
pub struct Row {
    pub run_id: String,
    pub timestamp_utc: String,
    pub machine_id: String,
    /// `taskbench`, `tokio`, `go` or `java`.
    pub implementation: String,
    pub benchmark: String,
    /// `A`, `B`, `C`, `D(A)`, `D(B)`, `E`, `tokio`, `go`, `java`.
    pub strategy: String,
    pub reservation_bytes: Option<u64>,
    pub stack_bytes: Option<u64>,
    pub threads: Option<u64>,
    pub tasks: Option<u64>,
    /// Benchmark parameters as `key=value;key=value`.
    pub params: String,
    pub metric: String,
    /// Numeric value; empty for text-only rows (machine specification, notes).
    pub value: Option<f64>,
    pub unit: String,
    pub status: Status,
    pub note: String,
}

/// Quotes a field when it contains a comma, quote, CR or LF (RFC 4180).
pub fn escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn opt<T: ToString>(v: &Option<T>) -> String {
    v.as_ref().map(|x| x.to_string()).unwrap_or_default()
}

/// Formats a value without exponent noise: integers print as integers.
pub fn fmt_value(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else if v.is_finite() {
        format!("{v:.3}")
    } else {
        String::new()
    }
}

impl Row {
    /// Renders the row as one CSV line without the trailing newline.
    pub fn to_line(&self) -> String {
        let fields = [
            SCHEMA_VERSION.to_string(),
            escape(&self.run_id),
            escape(&self.timestamp_utc),
            escape(&self.machine_id),
            escape(&self.implementation),
            escape(&self.benchmark),
            escape(&self.strategy),
            opt(&self.reservation_bytes),
            opt(&self.stack_bytes),
            opt(&self.threads),
            opt(&self.tasks),
            escape(&self.params),
            escape(&self.metric),
            self.value.map(fmt_value).unwrap_or_default(),
            escape(&self.unit),
            self.status.as_str().to_string(),
            escape(&self.note),
        ];
        fields.join(",")
    }
}

/// Appends rows to a CSV file.
pub struct CsvWriter {
    file: File,
}

impl CsvWriter {
    /// Opens `path` for appending, creating it and its parent directory; writes the
    /// header if the file is empty.
    pub fn open_append(path: &Path) -> io::Result<CsvWriter> {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)?;
            }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        if file.metadata()?.len() == 0 {
            writeln!(file, "{HEADER}")?;
        }
        Ok(CsvWriter { file })
    }

    /// Writes one row and flushes, so that a crash later in the run keeps it.
    pub fn write(&mut self, row: &Row) -> io::Result<()> {
        writeln!(self.file, "{}", row.to_line())?;
        self.file.flush()
    }
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

/// Formats seconds since the Unix epoch as an ISO 8601 UTC timestamp.
pub fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to (year, month, day); Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Row {
        Row {
            run_id: "r1".into(),
            timestamp_utc: "2026-10-12T00:00:00Z".into(),
            machine_id: "dev".into(),
            implementation: "taskbench".into(),
            benchmark: "spawn-join".into(),
            strategy: "B".into(),
            reservation_bytes: Some(1 << 20),
            stack_bytes: None,
            threads: Some(4),
            tasks: Some(1000),
            params: "rounds=1000;batch=1".into(),
            metric: "spawn_join_p50".into(),
            value: Some(812.0),
            unit: "ns".into(),
            status: Status::Ok,
            note: "a, \"quoted\" note".into(),
        }
    }

    #[test]
    fn escaping() {
        assert_eq!(escape("plain"), "plain");
        assert_eq!(escape("a,b"), "\"a,b\"");
        assert_eq!(escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn row_has_header_arity() {
        let line = sample().to_line();
        // Count fields with a tiny RFC 4180 reader.
        let mut fields = 1;
        let mut in_quotes = false;
        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => fields += 1,
                _ => {}
            }
        }
        assert_eq!(fields, HEADER.split(',').count());
        assert!(line.starts_with("1,r1,2026-10-12T00:00:00Z,dev,taskbench,spawn-join,B,1048576,,4,1000,"));
        assert!(line.ends_with(",812,ns,ok,\"a, \"\"quoted\"\" note\""));
    }

    #[test]
    fn writer_appends_header_once() {
        let dir = std::env::temp_dir().join(format!("taskbench-csv-{}", std::process::id()));
        let path = dir.join("out.csv");
        let _ = std::fs::remove_file(&path);
        {
            let mut w = CsvWriter::open_append(&path).unwrap();
            w.write(&sample()).unwrap();
        }
        {
            let mut w = CsvWriter::open_append(&path).unwrap();
            w.write(&sample()).unwrap();
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], HEADER);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn values_and_time() {
        assert_eq!(fmt_value(3.0), "3");
        assert_eq!(fmt_value(2.5), "2.500");
        assert_eq!(fmt_value(f64::NAN), "");
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_791_763_200), "2026-10-12T00:00:00Z");
    }
}

use std::{fmt::Display, io::{Error, ErrorKind}, time::SystemTime};

use color_eyre::Result;
use rusqlite::{params, types::FromSql, Connection};
use chrono::DateTime;

static DB_PATH: &str = "./data.db";

pub struct DBRow {
    timestamp: u64,
    topic: String,
    value: String,
}

impl From<&DBRow> for String {
    fn from(value: &DBRow) -> Self {
        let limited_db_row = value.fix_col_lengths(20);
        let timestamp = DateTime::from_timestamp(
            value.timestamp as i64,
            0
        ).unwrap_or(SystemTime::now().into());

        format!(
            "| {} | {} | {}",
            timestamp.naive_local().to_string(),
            limited_db_row.topic,
            limited_db_row.value,
        )
    }
}

impl Clone for DBRow {
    fn clone(&self) -> Self {
        DBRow {
           timestamp: self.timestamp,
           value: self.value.to_owned(),
           topic: self.topic.to_owned(),
        }
    }
}

impl Display for DBRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} -> {} -> {}", self.timestamp, self.value, self.topic))
    }
}

enum ColumnKind {
    FLOAT(f64),
    STRING(String),
}

impl Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::FLOAT(val) => f.write_str(&format!("{:.2}",val)),
            ColumnKind::STRING(val) => f.write_str(&format!("{}", val)),
        }
    }
}

impl FromSql for ColumnKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        if let Ok(val) = value.as_f64() {
            Ok(ColumnKind::FLOAT(val))
        } else {
            let bytes = value.as_bytes().unwrap_or(&[]);
            let val = String::from_utf8(bytes.into());
            Ok(ColumnKind::STRING(val.unwrap_or("No value".to_owned())))
        }
    }
}

pub fn delete_row_from_table (row: &DBRow, table_name: &str) -> Result<usize> {
    let conn = Connection::open(DB_PATH)?;
    let query = format!(
        "DELETE FROM {} WHERE timestamp = ?1 and topic = ?2 and value = ?3;",
        table_name
    );

    let rows_changed = conn.execute(&query, params![row.timestamp, row.topic, row.value])?;

    Ok(rows_changed)
}

pub fn get_all_from_table (table_name: &str) -> Result<Vec<DBRow>> {
    let conn = Connection::open(DB_PATH)?;
    let mut statement = conn.prepare(&format!("SELECT * FROM {};", table_name))?;

    let rows_iter = statement.query_map([], |row| {
        let val = row.get::<usize, ColumnKind>(2).unwrap_or(ColumnKind::FLOAT(0.));
        let curr_row = DBRow{
            timestamp: row.get(0).unwrap_or(0),
            topic: row.get(1).unwrap_or("".to_owned()),
            value: format!("{}", val),
        };
        Ok(curr_row)
    })?;

    let rows: Result<Vec<DBRow>, rusqlite::Error> = rows_iter.into_iter().collect();

    if let Ok(valid_rows) = rows {
        return Ok(valid_rows)
    }

    return Err(Error::new(ErrorKind::Other, "Something went wrong while getting data from the table").into());
}

fn fix_str_len(string: &str, len: usize) -> String {
    let mut new_string = String::new();
    let mut chars = string.chars();
    // let mut char_indicies = string.char_indices();
    for _ in 0..len {
        let char = chars.nth(0);
        if let Some(char) = char {
            new_string.push(char);
        } else {
            new_string.push(' ');
        }
    }

    new_string
}

impl DBRow {
    fn fix_col_lengths (&self, len: usize) -> Self {
        DBRow{
            timestamp: self.timestamp,
            topic: fix_str_len(&self.topic, len),
            value: fix_str_len(&self.value, len),
        }
    }
}

pub fn get_tables () -> Result<Vec<String>> {
    let conn = Connection::open(DB_PATH)?;
    let mut statement = conn.prepare("SELECT name FROM sqlite_master WHERE type='table';")?;
    let tables_iter = statement
        .query_map([], |row| { row.get::<usize, String>(0) })?;

    let tables: Result<Vec<String>, rusqlite::Error> = tables_iter.into_iter().collect();

    if let Ok(tables_vec) = tables {
        return Ok(tables_vec)
    }

    Err(Error::new(
        ErrorKind::Other, "Could not get db rows."
    ).into())
}

pub fn setup_db () -> Result<()> {
    let connection = Connection::open(DB_PATH)?;
    connection.execute("
        CREATE TABLE if not exists MEASUREMENTS (
                timestamp int,
                topic varchar(255),
                value float
        )
        ",
        (),
    )?;

    connection.execute("
        CREATE TABLE if not exists LOGS (
                timestamp int,
                topic varchar(255),
                value varchar(255)
        )
        ",
        (),
    )?;
    Ok(())
}

use std::{fmt::Display, io::{Error, ErrorKind}};
static DB_PATH: &str = "./data.db";

use color_eyre::Result;
use rusqlite::{params, Connection};

pub struct DBRow {
    timestamp: u64,
    topic: String,
    value: String,
}

impl From<&DBRow> for String {
    fn from(value: &DBRow) -> Self {
        format!("{} -> {} -> {}", value.timestamp, value.value, value.topic)
    }
}

impl Clone for DBRow {
    fn clone(&self) -> Self {
        DBRow {
           timestamp: self.timestamp,
           topic: self.topic.to_owned(),
           value: self.value.to_owned()
        }
    }
}

impl Display for DBRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} -> {} -> {}", self.timestamp, self.value, self.topic))
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
        let val = row.get::<usize, f32>(2).unwrap_or(0.);
        let curr_row = DBRow{
            timestamp: row.get(0).unwrap_or(0),
            topic: row.get(1).unwrap_or("".to_owned()),
            value: format!("{:.2}", val),
        };
        Ok(curr_row)
    })?;

    let rows: Result<Vec<DBRow>, rusqlite::Error> = rows_iter.into_iter().collect();

    if let Ok(valid_rows) = rows {
        return Ok(valid_rows)
    }

    return Err(Error::new(ErrorKind::Other, "Something went wrong while getting data from the table").into());
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

// Monitor in progress disk repairs
extern crate rusqlite;
extern crate time;

use time::Timespec;
use self::rusqlite::{Connection, Result};

#[derive(Debug)]
struct DiskRepairTicket {
    id: i32,
    name: String,
    time_created: Timespec,
    data: Option<Vec<u8>>,
}

pub fn create_repair_database(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE repairs (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL,
                  time_created    TEXT NOT NULL,
                  data            BLOB
                  )",
        &[],
    )?;
}

pub fn get_outstanding_repair_tickets(db_path: &Path) -> Result<()> {
    let me = Person {
        id: 0,
        name: "Steven".to_string(),
        time_created: time::get_time(),
        data: None,
    };
    conn.execute(
        "INSERT INTO repairs (name, time_created, data)
                  VALUES (?1, ?2, ?3)",
        &[&me.name, &me.time_created, &me.data],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, name, time_created, data FROM repairs",
    )?;
    let person_iter = stmt.query_map(&[], |row| {
        Person {
            id: row.get(0),
            name: row.get(1),
            time_created: row.get(2),
            data: row.get(3),
        }
    })?;

    for person in person_iter {
        println!("Found person {:?}", person?());
    }
}

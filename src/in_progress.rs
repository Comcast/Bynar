// Monitor in progress disk repairs
extern crate rusqlite;
extern crate time;

use std::path::Path;

use self::time::Timespec;
use self::rusqlite::{Connection, Result};

#[derive(Debug)]
pub struct DiskRepairTicket {
    pub id: i32,
    pub ticket_id: String,
    pub time_created: Timespec,
    pub disk_path: String,
}

pub fn create_repair_database(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    debug!("Opening or creating repairs table if needed");
    conn.execute(
        "CREATE TABLE if not exists repairs (
                  id              INTEGER PRIMARY KEY,
                  ticket_id       TEXT NOT NULL,
                  time_created    TEXT NOT NULL,
                  disk_path       TEXT NOT NULL)",
        &[],
    )?;
    Ok(conn)
}

pub fn record_new_repair_ticket(
    conn: &Connection,
    ticket_id: &str,
    disk_path: &Path,
) -> Result<()> {
    let ticket = DiskRepairTicket {
        id: 0,
        ticket_id: ticket_id.into(),
        time_created: time::get_time(),
        disk_path: disk_path.to_string_lossy().into_owned(),
    };
    debug!("Recording new repair ticket: {:?}", ticket);
    conn.execute(
        "INSERT INTO repairs (ticket_id, time_created, disk_path)
                  VALUES (?1, ?2, ?3)",
        &[
            &ticket.ticket_id,
            &ticket.time_created,
            &ticket.disk_path,
        ],
    )?;
    Ok(())
}

pub fn get_outstanding_repair_tickets(conn: &Connection) -> Result<Vec<DiskRepairTicket>> {
    let mut tickets: Vec<DiskRepairTicket> = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, ticket_id, time_created, disk_path FROM repairs",
    )?;
    let repair_iter = stmt.query_map(&[], |row| {
        DiskRepairTicket {
            id: row.get(0),
            ticket_id: row.get(1),
            time_created: row.get(2),
            disk_path: row.get(3),
        }
    })?;

    for repair in repair_iter {
        tickets.push(repair?);
    }
    Ok(tickets)
}

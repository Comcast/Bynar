// Monitor in progress disk repairs
extern crate rusqlite;
extern crate time;

use std::path::Path;

use self::time::Timespec;
use self::rusqlite::{Connection, Result};

#[cfg(test)]
mod tests {
    extern crate mktemp;

    use std::path::Path;

    use self::mktemp::Temp;

    #[test]
    fn test_in_progress() {
        let temp_dir = Temp::new_dir().expect("mktemp creation failed");
        let mut db_file = temp_dir.to_path_buf();
        db_file.push("test_db.sqlite3");

        let conn = super::create_repair_database(&db_file).expect("sqlite3 creation failed");
        super::record_new_repair_ticket(&conn, "001", &Path::new("/dev/sda"))
            .expect("Create repair ticket failed");
        let result = super::is_disk_in_progress(&conn, &Path::new("/dev/sda"))
            .expect("failed to query disk in progress");
        println!(
            "Outstanding repair tickets: {:?}",
            super::get_outstanding_repair_tickets(&conn)
        );

        assert!(result);
    }
}

#[derive(Debug)]
pub struct DiskRepairTicket {
    pub id: i32,
    pub ticket_id: String,
    pub time_created: Timespec,
    pub disk_path: String,
}

pub fn connect_to_repair_database(db_path: &Path) -> Result<Connection> {
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

/// Create a new repair ticket
pub fn record_new_repair_ticket(
    conn: &Connection,
    ticket_id: &str,
    disk_path: &Path,
) -> Result<()> {
    debug!(
        "Recording new repair ticket: id: {}, disk_path: {}",
        ticket_id,
        disk_path.display()
    );
    conn.execute(
        "INSERT INTO repairs (ticket_id, time_created, disk_path)
                  VALUES (?1, ?2, ?3)",
        &[
            &ticket_id.to_string(),
            &time::get_time(),
            &disk_path.to_string_lossy().into_owned(),
        ],
    )?;
    Ok(())
}

/// Check and return if a disk is in the database and awaiting repairs
pub fn is_disk_in_progress(conn: &Connection, dev_path: &Path) -> Result<bool> {
    debug!(
        "Searching for repair ticket for disk: {}",
        dev_path.display()
    );
    let mut stmt = conn.prepare(
        "SELECT id, ticket_id, time_created, disk_path FROM repairs where disk_path=?",
    )?;
    let in_progress = stmt.exists(&[&dev_path.to_string_lossy().into_owned()])?;
    Ok(in_progress)
}

/// Gather all the outstanding repair tickets
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

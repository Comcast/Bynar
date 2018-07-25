// Monitor in progress disk repairs
extern crate rusqlite;
extern crate time;

use std::path::{Path, PathBuf};
use std::str::FromStr;

use test_disk;

use self::rusqlite::{Connection, Result};
use self::time::Timespec;

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use std::path::Path;

    #[test]
    fn test_in_progress() {
        // FIX THIS
        let db_file = TempDir::new("test_db.sqlite3").expect("Temp file creation failed");

        let conn =
            super::connect_to_repair_database(db_file.path()).expect("sqlite3 creation failed");
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
    // TODO: should this be broken out into 2 tables,
    // 1 for repairs and 1 for state machine?
    conn.execute(
        "CREATE TABLE if not exists repairs (
                  id              INTEGER PRIMARY KEY,
                  ticket_id       TEXT,
                  time_created    TEXT,
                  disk_path       TEXT NOT NULL,
                  smart_passed    BOOLEAN,
                  mount_path      TEXT,
                  state           TEXT)",
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

pub fn resolve_ticket(conn: &Connection, ticket_id: &str) -> Result<()> {
    debug!("Resolving ticket: {}", ticket_id);
    conn.execute(
        "DELETE FROM repairs where ticket_id=?",
        &[&ticket_id.to_string()],
    )?;
    Ok(())
}

/// Check and return if a disk is in the database and awaiting repairs
pub fn is_disk_in_progress(conn: &Connection, dev_path: &Path) -> Result<bool> {
    debug!(
        "Searching for repair ticket for disk: {}",
        dev_path.display()
    );
    let mut stmt = conn
        .prepare("SELECT id, ticket_id, time_created, disk_path FROM repairs where disk_path=?")?;
    let in_progress = stmt.exists(&[&dev_path.to_string_lossy().into_owned()])?;
    Ok(in_progress)
}

/// Gather all the outstanding repair tickets
pub fn get_outstanding_repair_tickets(conn: &Connection) -> Result<Vec<DiskRepairTicket>> {
    let mut tickets: Vec<DiskRepairTicket> = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, ticket_id, time_created, disk_path FROM repairs where ticket_id IS NOT NULL",
    )?;
    let repair_iter = stmt.query_map(&[], |row| DiskRepairTicket {
        id: row.get(0),
        ticket_id: row.get(1),
        time_created: row.get(2),
        disk_path: row.get(3),
    })?;

    for repair in repair_iter {
        tickets.push(repair?);
    }
    Ok(tickets)
}

pub fn get_mount_location(conn: &Connection, dev_path: &Path) -> Result<PathBuf> {
    debug!("Searching smart results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT mount_path FROM repairs where disk_path=?")?;
    let mount_path = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
        let row: String = row.get(0);
        PathBuf::from(row)
    })?;
    Ok(mount_path)
}

pub fn get_smart_result(conn: &Connection, dev_path: &Path) -> Result<bool> {
    debug!("Searching smart results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT smart_passed FROM repairs where disk_path=?")?;
    let passed = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| row.get(0))?;
    Ok(passed)
}

pub fn get_state(conn: &Connection, dev_path: &Path) -> Result<Option<test_disk::State>> {
    debug!("Searching state results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT state FROM repairs where disk_path=?")?;
    let state_exists = stmt.exists(&[&dev_path.to_string_lossy().into_owned()])?;
    if state_exists {
        let mut stmt = conn.prepare("SELECT state FROM repairs where disk_path=?")?;
        let state = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
            let state: String = row.get(0);
            state
        })?;

        return Ok(Some(test_disk::State::from_str(&state).unwrap()));
    }

    Ok(None)
}

pub fn save_mount_location(conn: &Connection, dev_path: &Path, mount_path: &Path) -> Result<()> {
    debug!(
        "Saving mount path for {}: {}",
        dev_path.display(),
        mount_path.display()
    );
    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    match stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        true => {
            // It exists so we update
            let mut stmt = conn.prepare("Update repairs set mount_path=? where disk_path=?")?;
            stmt.execute(&[
                &mount_path.to_string_lossy().into_owned(),
                &dev_path.to_string_lossy().into_owned(),
            ])?;
        }
        false => {
            // It does not exist so we insert
            conn.execute(
                "INSERT INTO repairs (mount_path, disk_path)
                  VALUES (?1, ?2)",
                &[
                    &mount_path.to_string_lossy().into_owned(),
                    &dev_path.to_string_lossy().into_owned(),
                ],
            )?;
        }
    }

    Ok(())
}

pub fn save_smart_results(conn: &Connection, dev_path: &Path, smart_passed: bool) -> Result<()> {
    debug!(
        "Saving smart results for {} passed: {}",
        dev_path.display(),
        smart_passed.to_string()
    );
    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    match stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        true => {
            // It exists so we update
            let mut stmt = conn.prepare("Update repairs set smart_passed=? where disk_path=?")?;
            stmt.execute(&[&smart_passed, &dev_path.to_string_lossy().into_owned()])?;
        }
        false => {
            // It does not exist so we insert
            let mut stmt =
                conn.prepare("Insert INTO repairs (smart_passed, disk_path) VALUES (?1, ?2)")?;
            stmt.execute(&[&smart_passed, &dev_path.to_string_lossy().into_owned()])?;
        }
    }

    Ok(())
}

pub fn save_state(conn: &Connection, dev_path: &Path, state: test_disk::State) -> Result<()> {
    debug!("Saving state for {}: {}", dev_path.display(), state);

    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    match stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        true => {
            debug!("Updating state for {}", dev_path.display());
            // It exists so we update
            let mut stmt = conn.prepare("Update repairs set state=? where disk_path=?")?;
            stmt.execute(&[&state.to_string(), &dev_path.to_string_lossy().into_owned()])?;
        }
        false => {
            debug!("Inserting state for {}", dev_path.display());
            // It does not exist so we insert
            conn.execute(
                "INSERT INTO repairs (state, disk_path) VALUES (?1, ?2)",
                &[&state.to_string(), &dev_path.to_string_lossy().into_owned()],
            )?;
        }
    }
    Ok(())
}

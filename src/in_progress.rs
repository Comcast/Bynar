/// Monitor in progress disk repairs
extern crate chrono;
extern crate helpers;
extern crate postgres;
extern crate postgres_shared;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rusqlite;
extern crate time;

use test_disk;

use self::chrono::offset::Utc;
use self::chrono::DateTime;
use self::helpers::{error::*, host_information::Host as MyHost};
use self::postgres::{
    params::ConnectParams, params::Host, Connection as pConnection, Result as pResult,
};
use self::r2d2::{Pool, PooledConnection};
use self::r2d2_postgres::{PostgresConnectionManager as ConnectionManager, TlsMode};
use self::rusqlite::Connection;
use self::test_disk::{BlockDevice, State};
use self::time::Timespec;
use super::DBConfig;
use std::fmt::{Display, Formatter, Result as fResult};
use std::path::{Path, PathBuf};
use std::process::id;
use std::str::FromStr;
use std::time::Duration;

#[cfg(test)]
mod tests {
    extern crate log;
    extern crate tempdir;
    use self::tempdir::TempDir;
    use simplelog::{Config, TermLogger};
    use std::path::Path;
    use std::process::id;
    use ConfigSettings;

    #[test]
    fn test_in_progress() {
        let sql_dir = TempDir::new("bynar").expect("Temp file creation failed");
        let db_path = sql_dir.path().join("in_progress.sqlite3");

        let conn = super::connect_to_repair_database(&db_path).expect("sqlite3 creation failed");
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

    #[test]
    fn test_new_host() {
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();
        let info = super::MyHost::new().unwrap();
        println!("{:#?}", info);
    }

    #[test]
    fn test_update_storage_info() {
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();
        let config_dir = Path::new("/newDevice/tests/");
        let config: ConfigSettings =
            super::helpers::load_config(config_dir, "bynar.json").expect("Failed to load config");
        let db_config = config.database;
        let conn: super::pConnection = super::connect_to_database(&db_config).unwrap();

        let pid = id();
        let info = super::MyHost::new().unwrap();
        let result = super::update_storage_info(&info, pid, &conn).expect(
            "Failed to update
                storage details",
        );

        println!("Successfully updated storage details");
        assert!(result);

        // close database connection
        super::disconnect_database(conn).expect("failed to close DB connection");
    }

}

#[derive(Debug)]
pub struct DiskRepairTicket {
    pub id: i32,
    pub ticket_id: String,
    pub time_created: Timespec,
    pub disk_path: String,
}

pub fn connect_to_repair_database(db_path: &Path) -> BynarResult<Connection> {
    debug!("Opening or creating repairs table if needed");
    let conn = Connection::open(db_path)?;
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
) -> BynarResult<()> {
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

pub fn resolve_ticket(conn: &Connection, ticket_id: &str) -> BynarResult<()> {
    debug!("Resolving ticket: {}", ticket_id);
    conn.execute(
        "DELETE FROM repairs where ticket_id=?",
        &[&ticket_id.to_string()],
    )?;
    Ok(())
}

/// Check and return if a disk is in the database and awaiting repairs
pub fn is_disk_in_progress(conn: &Connection, dev_path: &Path) -> BynarResult<bool> {
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
pub fn get_outstanding_repair_tickets(conn: &Connection) -> BynarResult<Vec<DiskRepairTicket>> {
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

pub fn get_mount_location(conn: &Connection, dev_path: &Path) -> BynarResult<PathBuf> {
    debug!("Searching smart results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT mount_path FROM repairs where disk_path=?")?;
    let mount_path = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
        let row: String = row.get(0);
        PathBuf::from(row)
    })?;
    Ok(mount_path)
}
/*
pub fn get_smart_result(conn: &Connection, dev_path: &Path) -> BynarResult<bool> {
    debug!("Searching smart results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT smart_passed FROM repairs where disk_path=?")?;
    let passed = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
        row.get(0)
    })?;
    Ok(passed)
}

pub fn get_state(conn: &Connection, dev_path: &Path) -> BynarResult<Option<test_disk::State>> {
    debug!("Searching state results for disk: {}", dev_path.display());
    let mut stmt = conn.prepare("SELECT state FROM repairs where disk_path=?")?;
    let state_exists = stmt.exists(&[&dev_path.to_string_lossy().into_owned()])?;
    if state_exists {
        let mut stmt = conn.prepare("SELECT state FROM repairs where disk_path=?")?;
        let state = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
            let state: String = row.get(0);
            state
        })?;
        debug!("Found state: {}", state);

        return Ok(Some(
            test_disk::State::from_str(&state).unwrap_or(test_disk::State::Unscanned),
        ));
    }

    Ok(None)
}
*/
pub fn save_mount_location(
    conn: &Connection,
    dev_path: &Path,
    mount_path: &Path,
) -> BynarResult<()> {
    debug!(
        "Saving mount path for {}: {}",
        dev_path.display(),
        mount_path.display()
    );
    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    if stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        // It exists so we update
        let mut stmt = conn.prepare("Update repairs set mount_path=? where disk_path=?")?;
        stmt.execute(&[
            &mount_path.to_string_lossy().into_owned(),
            &dev_path.to_string_lossy().into_owned(),
        ])?;
    } else {
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

    Ok(())
}

/*
pub fn save_smart_results(
    conn: &Connection,
    dev_path: &Path,
    smart_passed: bool,
) -> BynarResult<()> {
    debug!(
        "Saving smart results for {} passed: {}",
        dev_path.display(),
        smart_passed.to_string()
    );
    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    if stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        // It exists so we update
        let mut stmt = conn.prepare("Update repairs set smart_passed=? where disk_path=?")?;
        stmt.execute(&[&smart_passed, &dev_path.to_string_lossy().into_owned()])?;
    } else {
        // It does not exist so we insert
        let mut stmt =
            conn.prepare("Insert INTO repairs (smart_passed, disk_path) VALUES (?1, ?2)")?;
        stmt.execute(&[&smart_passed, &dev_path.to_string_lossy().into_owned()])?;
    }

    Ok(())
}
pub fn save_state(conn: &Connection, dev_path: &Path, state: test_disk::State) -> BynarResult<()> {
    debug!("Saving state for {}: {}", dev_path.display(), state);

    // First check if a row exists with this disk
    let mut stmt = conn.prepare("SELECT * FROM repairs where disk_path=?")?;
    if stmt.exists(&[&dev_path.to_string_lossy().into_owned()])? {
        debug!("Updating state for {}", dev_path.display());
        // It exists so we update
        let mut stmt = conn.prepare("Update repairs set state=? where disk_path=?")?;
        stmt.execute(&[&state.to_string(), &dev_path.to_string_lossy().into_owned()])?;
    } else {
        debug!("Inserting state for {}", dev_path.display());
        // It does not exist so we insert
        conn.execute(
            "INSERT INTO repairs (state, disk_path) VALUES (?1, ?2)",
            &[&state.to_string(), &dev_path.to_string_lossy().into_owned()],
        )?;
    }
    Ok(())
}
*/
#[derive(Debug)]
pub struct DiskInfo {
    pub disk_id: u32,
    pub storage_detail_id: u32,
    pub disk_path: PathBuf,
    pub mount_path: PathBuf,
    pub disk_name: String,
    pub disk_uuid: Option<String>,
}

impl DiskInfo {
    pub fn new(
        disk_name: String,
        disk_path: PathBuf,
        mount_path: PathBuf,
        storage_detail_id: u32,
    ) -> DiskInfo {
        DiskInfo {
            disk_id: 0,
            disk_name,
            disk_path,
            mount_path,
            storage_detail_id,
            disk_uuid: None,
        }
    }

    pub fn set_disk_id(&mut self, disk_id: u32) {
        self.disk_id = disk_id;
    }
    pub fn set_disk_uuid(&mut self, disk_uuid: String) {
        self.disk_uuid = Some(disk_uuid);
    }
}

#[derive(Debug)]
pub struct HostDetailsMapping {
    pub entry_id: u32,
    pub region_id: u32,
    pub storage_detail_id: u32,
}

impl HostDetailsMapping {
    pub fn new(entry_id: u32, region_id: u32, storage_detail_id: u32) -> HostDetailsMapping {
        HostDetailsMapping {
            entry_id,
            region_id,
            storage_detail_id,
        }
    }
}

#[derive(Debug)]
pub struct OperationInfo {
    pub operation_id: u32,
    pub host_details: HostDetailsMapping,
    pub disk_id: u32,
    pub behalf_of: Option<String>,
    pub reason: Option<String>,
    pub start_time: DateTime<Utc>,
    pub snapshot_time: DateTime<Utc>,
    pub done_time: Option<DateTime<Utc>>,
}

impl OperationInfo {
    fn new(host_details: HostDetailsMapping, disk_id: u32) -> OperationInfo {
        OperationInfo {
            operation_id: 0,
            host_details,
            disk_id,
            behalf_of: None,
            reason: None,
            start_time: Utc::now(),
            snapshot_time: Utc::now(),
            done_time: None,
        }
    }
}

#[derive(Debug)]
pub enum OperationType {
    DiskAdd,
    DiskReplace,
    DiskRemove,
    WaitForReplacement,
    Evaluation,
}

impl Display for OperationType {
    fn fmt(&self, f: &mut Formatter) -> fResult {
        let message = match *self {
            OperationType::DiskAdd => "diskadd",
            OperationType::DiskReplace => "diskreplace",
            OperationType::DiskRemove => "diskremove",
            OperationType::WaitForReplacement => "waitforreplacement",
            OperationType::Evaluation => "evaluation",
        };
        write!(f, "{}", message)
    }
}

#[derive(Debug)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Complete,
}

impl Display for OperationStatus {
    fn fmt(&self, f: &mut Formatter) -> fResult {
        let message = match *self {
            OperationStatus::Pending => "pending",
            OperationStatus::InProgress => "in_progress",
            OperationStatus::Complete => "complete",
        };
        write!(f, "{}", message)
    }
}

#[derive(Debug)]
pub struct OperationDetail {
    pub op_detail_id: u32,
    pub operation_id: u32,
    pub op_type: OperationType,
    pub status: OperationStatus,
    pub tracking_id: Option<u32>,
    pub start_time: DateTime<Utc>,
    pub snapshot_time: DateTime<Utc>,
    pub done_time: Option<DateTime<Utc>>,
}

impl OperationDetail {
    fn new(operation_id: u32, op_type: OperationType) -> OperationDetail {
        OperationDetail {
            op_detail_id: 0,
            operation_id,
            op_type,
            status: OperationStatus::Pending,
            tracking_id: None,
            start_time: Utc::now(),
            snapshot_time: Utc::now(),
            done_time: None,
        }
    }
    fn set_operation_detail_id(&mut self, op_detail_id: u32) {
        self.op_detail_id = op_detail_id;
    }

    fn set_tracking_id(&mut self, tracking_id: u32) {
        self.tracking_id = Some(tracking_id);
    }

    fn set_done_time(&mut self, done_time: DateTime<Utc>) {
        self.done_time = Some(done_time);
    }

    fn set_operation_status(&mut self, status: OperationStatus) {
        self.status = status;
    }
}

/// Reads the config file to establish a pool of database connections
pub fn create_db_connection_pool(db_config: &DBConfig) -> BynarResult<Pool<ConnectionManager>> {
    debug!(
        "Establishing a connection to database {} at {}:{} using {}",
        db_config.dbname, db_config.endpoint, db_config.port, db_config.username
    );
    let connection_params = ConnectParams::builder()
        .user(&db_config.username, Some(&db_config.password))
        .port(db_config.port)
        .database(&db_config.dbname)
        .build(Host::Tcp(db_config.endpoint.to_string()));
    let manager = ConnectionManager::new(connection_params, TlsMode::None)?;
    let db_pool = Pool::builder()
        .max_size(10)
        .connection_timeout(Duration::from_secs(300))
        .build(manager)?;
    Ok(db_pool)
}

/// return one connection from the pool
pub fn get_connection_from_pool(
    pool: &Pool<ConnectionManager>,
) -> BynarResult<PooledConnection<ConnectionManager>> {
    let connection = pool.get()?;
    Ok(connection)
}

/// closes the connection. Should be called for every corresponding call
/// to get_connection_from_pool()
pub fn drop_connection(conn: PooledConnection<ConnectionManager>) -> BynarResult<()> {
    Ok(drop(conn))
}

/// Should be called when bynar daemon first starts up
/// Returns whether or not all steps in this call have been successful
/// TODO: return conn, entry_id, region_id, detail_id
pub fn update_storage_info(s_info: &MyHost, pool: &Pool<ConnectionManager>) -> BynarResult<bool> {
    debug!("Adding datacenter and host information to database");

    // Get a database connection
    let conn = get_connection_from_pool(pool)?;
    // get process id
    let pid = id();
    // extract ip address to a &str
    let ip_address: String = s_info.ip.to_string();
    let entry_id = register_to_process_manager(&conn, pid, &ip_address)?;
    let region_id = update_region(&conn, &s_info.region.clone())?;
    let detail_id = update_storage_details(&conn, &s_info, region_id)?;

    // Done, drop connection
    drop(conn);
    if entry_id == 0 || region_id == 0 || detail_id == 0 {
        error!("Failed to update storage information in the database");
    } else {
        // TODO: Add entry, region_id and storage_id to bynar_operations table
    }
    Ok(true)
}

/// responsible to store the pid, ip of the system on which bynar is running
fn register_to_process_manager(conn: &pConnection, pid: u32, ip: &str) -> pResult<u32> {
    debug!("Adding daemon details with pid {} to process manager", pid);
    let mut entry_id: u32 = 0;
    let stmt = format!(
        "SELECT entry_id FROM process_manager WHERE
    pid={} AND ip='{}'",
        pid, &ip
    );
    let stmt_query = conn.query(&stmt, &[])?;
    if let Some(_r) = stmt_query.into_iter().next() {
        // entry exists for this ip with this pid. Update status
        let update_stmt = format!(
            "UPDATE process_manager SET status='idle'
           WHERE pid={} AND ip='{}'",
            pid, &ip
        );
        conn.execute(&update_stmt, &[])?;
        let select_stmt = format!(
            "SELECT entry_id FROM process_manager WHERE pid
        = {} AND ip = '{}'",
            pid, &ip
        );
        let select_stmt_query = conn.query(&select_stmt, &[])?;
        if let Some(r) = select_stmt_query.into_iter().next() {
            let e: i32 = r.get("entry_id");
            entry_id = e as u32;
        }
    } else {
        // does not exist, insert
        let insert_stmt = format!(
            "INSERT INTO process_manager (pid, ip, status)
                            VALUES ({}, '{}', 'idle') RETURNING entry_id",
            pid, &ip
        );
        let insert_stmt_query = conn.query(&insert_stmt, &[])?;
        if let Some(r) = insert_stmt_query.into_iter().next() {
            let e: i32 = r.get("entry_id");
            entry_id = e as u32;
        }
    }
    Ok(entry_id)
}

/// Responsible to de-register itself when daemon exists
pub fn deregister_from_process_manager() -> pResult<()> {
    // DELETE FROM process_manager WHERE IP=<>
    Ok(())
}

// Checks for the region in the database, inserts if it does not exist
// and returns the region_id
fn update_region(conn: &pConnection, region: &str) -> pResult<u32> {
    let stmt = format!(
        "SELECT region_id FROM regions WHERE region_name = '{}'",
        region
    );
    let stmt_query = conn.query(&stmt, &[])?;
    let mut region_id: u32 = 0;

    if let Some(res) = stmt_query.into_iter().next() {
        // Exists, return region_id
        let id: i32 = res.get(0);
        region_id = id as u32;
    } else {
        // does not exist, insert
        debug!("Adding region {} to database", region);
        let stmt = format!(
            "INSERT INTO regions (region_name)
                            VALUES ('{}') RETURNING region_id",
            region
        );
        let stmt_query = conn.query(&stmt, &[])?;
        if let Some(res) = stmt_query.into_iter().next() {
            // Exists
            let id: i32 = res.get(0);
            region_id = id as u32;
        }
    }
    Ok(region_id)
}

fn update_storage_details(conn: &pConnection, s_info: &MyHost, region_id: u32) -> pResult<u32> {
    let stmt = format!(
        "SELECT storage_id FROM storage_types WHERE storage_type='{}'",
        s_info.storage_type
    );
    let stmt_query = conn.query(&stmt, &[])?;
    let mut storage_detail_id: u32 = 0;

    if let Some(r) = stmt_query.into_iter().next() {
        let sid: i32 = r.get("storage_id");
        let storage_id: u32 = sid as u32;

        // query if these storage details are already in DB
        let details_query = format!(
            "SELECT detail_id FROM storage_details WHERE storage_id = {}
            AND region_id = {} AND hostname = '{}'",
            storage_id, region_id, s_info.hostname
        );
        let details_query_exec = conn.query(&details_query, &[])?;
        if let Some(res) = details_query_exec.into_iter().next() {
            //Exists
            let sdi: i32 = res.get("detail_id");
            storage_detail_id = sdi as u32;
        } else {
            // TODO: modify when exact storage details are added

            let mut details_query = "INSERT INTO storage_details
            (storage_id, region_id, hostname"
                .to_string();
            if s_info.array_name.is_some() {
                details_query.push_str(", name_key1");
            }
            if s_info.pool_name.is_some() {
                details_query.push_str(", name_key2");
            }
            details_query.push_str(&format!(
                ") VALUES ({}, {}, '{}'",
                storage_id, region_id, s_info.hostname
            ));
            if let Some(ref array_name) = s_info.array_name {
                details_query.push_str(&format!(", '{}'", array_name));
            }
            if let Some(ref pool_name) = s_info.pool_name {
                details_query.push_str(&format!(", '{}'", pool_name));
            }
            details_query.push_str(") RETURNING detail_id");

            let dqr = conn.query(&details_query, &[])?;
            if let Some(res) = dqr.into_iter().next() {
                let sdi: i32 = res.get("detail_id");
                storage_detail_id = sdi as u32;
            } else {
                // failed to insert
                error!("Query to insert and retrive storage details failed");
            }
        }
    } else {
        error!("Storage type {} not in database", s_info.storage_type);
    }
    Ok(storage_detail_id)
}

// Inserts disk informatation record into bynar.disks and adds the disk_id to struct
pub fn add_disk_detail(conn: &pConnection, disk_info: &mut DiskInfo) -> pResult<bool> {
    let mut stmt = String::new();

    match disk_info.disk_id {
        0 => {
            // no disk_id present, add a new record
            stmt.push_str("INSERT INTO disks (storage_detail_id, disk_path, disk_name, mount_path");
            if disk_info.disk_uuid.is_some() {
                stmt.push_str(", disk_uuid");
            }
            stmt.push_str(&format!(
                ") VALUES ({}, {}, {}, {}",
                disk_info.storage_detail_id,
                disk_info.disk_path.to_string_lossy().into_owned(),
                disk_info.disk_name,
                disk_info.mount_path.to_string_lossy().into_owned()
            ));
            if let Some(ref uuid) = disk_info.disk_uuid {
                stmt.push_str(&format!(", {}", uuid));
            }

            stmt.push_str(") RETURNING disk_id");
        }
        _ => {
            // verify if all other details match, select disk_id to match with the
            // return from the insert stmt above
            stmt.push_str(&format!(
                "SELECT disk_id FROM disks WHERE disk_id = {} AND
                                    disk_name = {} AND disk_path = {} AND 
                                    mounth_path = {} AND 
                                    storage_detail_id = {}",
                disk_info.disk_id,
                disk_info.disk_name,
                disk_info.disk_path.to_string_lossy(),
                disk_info.mount_path.to_string_lossy(),
                disk_info.storage_detail_id
            ));
        }
    }
    let stmt_query = conn.query(&stmt, &[])?;

    if let Some(result) = stmt_query.into_iter().next() {
        let id: i32 = result.get("disk_id");
        disk_info.set_disk_id(id as u32);
        Ok(true)
    } else {
        error!("Failed to add disk information to database");
        Ok(false)
    }
}

// inserts the operation record and returns the operation_id
// If updating, performs the query and returns the given operation_id if successful update
pub fn add_or_update_operation(conn: &pConnection, op_info: &OperationInfo) -> pResult<u32> {
    let mut op_id: u32 = 0;
    let mut stmt = String::new();

    match op_info.operation_id {
        0 => {
            // no operation_id, validate new record input
            let host_info: &HostDetailsMapping = &op_info.host_details;
            if host_info.region_id == 0
                || host_info.entry_id == 0
                || host_info.storage_detail_id == 0
            {
                error!("Required region_id, storage_detail_id, entry_id");
                return Ok(0);
            }
            stmt.push_str(
                "INSERT INTO operations (
                                    region_id, storage_detail_id, entry_id
                                    start_time, disk_id",
            );

            if op_info.behalf_of.is_some() {
                stmt.push_str(", behalf_of");
            }
            if op_info.reason.is_some() {
                stmt.push_str(", reason");
            }

            stmt.push_str(")");

            stmt.push_str(&format!(
                " VALUES ({},{},{},{}, {}",
                host_info.region_id,
                host_info.storage_detail_id,
                host_info.entry_id,
                op_info.start_time,
                op_info.disk_id
            ));

            if let Some(ref behalf_of) = op_info.behalf_of {
                stmt.push_str(&format!(", {}", behalf_of));
            }
            if let Some(ref reason) = op_info.reason {
                stmt.push_str(&format!(", {}", reason));
            }

            stmt.push_str(") RETURNING operation_id");
        }
        _ => {
            // update existing record. Only snapshot_time and done_time
            // can be updated.
            stmt.push_str(&format!(
                "UPDATE operations SET (snapshot_time = {}",
                op_info.snapshot_time
            ));

            if let Some(d_time) = op_info.done_time {
                stmt.push_str(&format!(", done_time = {}", d_time));
            }
            stmt.push_str(&format!(") WHERE operation_id = {}", op_info.operation_id));

            op_id = op_info.operation_id;
        }
    }
    let stmt_query = conn.query(&stmt, &[])?;
    if let Some(result) = stmt_query.into_iter().next() {
        let id: i32 = result.get("operation_id");
        op_id = id as u32;
    } else {
        // failed to insert
        error!("Failed to update operation to database");
    }

    Ok(op_id)
}

pub fn add_or_update_operation_detail(
    conn: &pConnection,
    operation_detail: &mut OperationDetail,
) -> pResult<bool> {
    let mut stmt = String::new();
    let mut insert = false;
    match operation_detail.op_detail_id {
        0 => {
            // insert new detail record
            let mut type_id = 0;
            let stmt2 = format!(
                "SELECT type_id FROM operation_types WHERE
                                op_name={}",
                operation_detail.op_type
            );
            let stmt_query = conn.query(&stmt2, &[])?;
            if let Some(result) = stmt_query.into_iter().next() {
                type_id = result.get("type_id")
            }

            if type_id == 0 {
                error!("Failed to retrieve operation type ID");
                return Ok(false);
            }
            stmt.push_str(
                "INSERT INTO operation_details (operation_id, type_id,
                            status, start_time, snapshot_time",
            );
            if operation_detail.tracking_id.is_some() {
                stmt.push_str(", tracking_id");
            }
            if operation_detail.done_time.is_some() {
                stmt.push_str(", done_time");
            }

            stmt.push_str(&format!(
                " ) VALUES ({}, {}, {}, {}, {}",
                operation_detail.operation_id,
                type_id,
                operation_detail.status,
                operation_detail.start_time,
                operation_detail.snapshot_time
            ));

            if let Some(t_id) = operation_detail.tracking_id {
                stmt.push_str(&format!(", {}", t_id));
            }
            if let Some(done_time) = operation_detail.done_time {
                stmt.push_str(&format!(", {}", done_time));
            }
            stmt.push_str(") RETURNING operation_detail_id");
            insert = true;
        }
        _ => {
            // update existing detail record.
            // Only tracking_id, snapshot_time, done_time and status are update-able
            stmt.push_str(&format!(
                "UPDATE operation_details SET (snapshot_time = {}, 
                            status = {}",
                operation_detail.snapshot_time, operation_detail.status
            ));
            if let Some(t_id) = operation_detail.tracking_id {
                stmt.push_str(&format!(", tracking_id = {}", t_id));
            }
            if let Some(done_time) = operation_detail.done_time {
                stmt.push_str(&format!(", done_time = {}", done_time));
            }
            stmt.push_str(&format!(
                ") WHERE operation_detail_id = {}",
                operation_detail.op_detail_id
            ));
        }
    }

    let stmt_query = conn.query(&stmt, &[])?;
    if insert {
        if let Some(result) = stmt_query.into_iter().next() {
            let id: i32 = result.get("operation_detail_id");
            operation_detail.set_operation_detail_id(id as u32);
            Ok(true)
        } else {
            // failed to insert
            error!("Failed to add operation detail to database");
            Ok(false)
        }
    } else {
        Ok(true)
    }
}

pub fn save_state(
    pool: &Pool<ConnectionManager>,
    device_detail: &BlockDevice,
    state: State,
) -> BynarResult<()> {
    debug!(
        "Saving state as {} for device {}",
        state, device_detail.device.name
    );
    let conn = get_connection_from_pool(pool)?;

    if let Some(dev_id) = device_detail.device_database_id {
        // Device is in database, update the state. Start a transaction to roll back if needed.
        // transaction rolls back by default.
        let transaction = conn.transaction()?;
        let stmt = format!(
            "UPDATE disks SET state = {} WHERE disk_id={}",
            state, dev_id
        );
        let stmt_query = transaction.execute(&stmt, &[])?;
        info!(
            "Updated {} rows in database with state information",
            stmt_query
        );
        if stmt_query != 1 {
            // Only one device should  be updated. Rollback
            transaction.set_rollback();
            transaction.finish();
            Err(BynarError::new(
                "Attempt to update more than one device in database. Rolling back.".to_string(),
            ))
        } else {
            transaction.set_commit();
            transaction.finish();
            Ok(())
        }
    } else {
        // device is not in database. It should have been.
        Err(BynarError::new(format!(
            "Device {} for storage detail with id {} is not in database",
            device_detail.device.name, device_detail.storage_detail_id
        )))
    }
}

pub fn save_smart_result(
    pool: &Pool<ConnectionManager>,
    device_detail: &BlockDevice,
    smart_passed: bool,
) -> BynarResult<()> {
    debug!(
        "Saving smart check result as {} for device {}",
        smart_passed, device_detail.device.name
    );
    let conn = get_connection_from_pool(pool)?;

    if let Some(dev_id) = device_detail.device_database_id {
        // Device is in database, update smart_passed. Start a transaction to roll back if needed.
        // transaction rolls back by default.
        let transaction = conn.transaction()?;
        let stmt = format!(
            "UPDATE disks SET smart_passed = {} WHERE disk_id={}",
            smart_passed, dev_id
        );
        let stmt_query = transaction.execute(&stmt, &[])?;
        info!(
            "Updated {} rows in database with smart check result",
            stmt_query
        );
        if stmt_query != 1 {
            // Only one device should  be updated. Rollback
            transaction.set_rollback();
            transaction.finish();
            Err(BynarError::new(
                "Attempt to update more than one device in database. Rolling back.".to_string(),
            ))
        } else {
            transaction.set_commit();
            transaction.finish();
            Ok(())
        }
    } else {
        // device is not in database. It should have been.
        Err(BynarError::new(format!(
            "Device {} for storage detail with id {} is not in database",
            device_detail.device.name, device_detail.storage_detail_id
        )))
    }
}

/// Returns the state information from the database.
/// Returns error if no record of device is found in the database.
/// Returns the default state if state was not previously saved.
pub fn get_state(
    pool: &Pool<ConnectionManager>,
    device_detail: &BlockDevice,
) -> BynarResult<State> {
    debug!(
        "Retrieving state for device {} with storage detail id {} from DB",
        device_detail.device.name, device_detail.storage_detail_id
    );
    let conn = get_connection_from_pool(pool)?;

    if let Some(dev_id) = device_detail.device_database_id {
        let stmt = format!("SELECT state FROM devices WHERE device_id = {}", dev_id);
        let stmt_query = conn.query(&stmt, &[])?;
        if stmt_query.len() != 1 || stmt_query.is_empty() {
            Ok(State::Unscanned)
        } else {
            let row = stmt_query.get(0);
            let retrieved_state: String = row.get("state");
            Ok(State::from_str(&retrieved_state).unwrap_or(State::Unscanned))
        }
    } else {
        // No entry of this device in database table. Cannot get state information
        Err(BynarError::new(format!(
            "Device {} for storage detail {} is not in DB",
            device_detail.device.name, device_detail.storage_detail_id
        )))
    }
}

/// Returns whether smart checks have passed information from the database.
/// Returns error if no record of device is found in the database.
/// Returns false if not previously saved.
pub fn get_smart_result(
    pool: &Pool<ConnectionManager>,
    device_detail: &BlockDevice,
) -> BynarResult<bool> {
    debug!(
        "Retrieving smart check result for device {} with storage detail id {} from DB",
        device_detail.device.name, device_detail.storage_detail_id
    );
    let conn = get_connection_from_pool(pool)?;

    if let Some(dev_id) = device_detail.device_database_id {
        let stmt = format!(
            "SELECT smart_passed FROM devices WHERE device_id = {}",
            dev_id
        );
        let stmt_query = conn.query(&stmt, &[])?;
        if stmt_query.len() != 1 || stmt_query.is_empty() {
            // Query didn't return anything. Assume smart checks have not been done/passed
            Ok(false)
        } else {
            // got something from the database
            let row = stmt_query.get(0);
            let smart_passed = row.get("smart_passed");
            Ok(smart_passed)
        }
    } else {
        // No entry of this device in database table. Cannot get smart_cheks info
        Err(BynarError::new(format!(
            "Device {} for storage detail {} is not in DB",
            device_detail.device.name, device_detail.storage_detail_id
        )))
    }
}

// Monitor in progress disk repairs
extern crate postgres;
extern crate rusqlite;
extern crate time;
extern crate chrono;

use self::chrono::offset::Utc;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::net::IpAddr;
use std::fmt::{Display, Result as fResult, Formatter};
use self::postgres::{Connection as pConnection, Result as pResult, TlsMode};
use self::rusqlite::{Connection, Result};
use self::time::Timespec;
use self::chrono::DateTime;

use test_disk;


#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use std::path::Path;
    use std::process::id;
    #[test]
    fn test_in_progress() {
        let sql_dir = TempDir::new("bynar").expect("Temp file creation failed");
        let db_path = sql_dir.path().join("in_progress.sqlite3");

        let conn =
            super::connect_to_repair_database(&db_path).expect("sqlite3 creation failed");
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
    fn test_update_storage_info() {
        let configFile = "/tmp/dbconfig.json".to_string();
        let connection_string = read_db_config(&configFile);

        println!("Connection string is {}", connection_string);
        let pid = id();
        let info = HostDetails {
            region: String::from("test-region"),
            ip: IpAddr::from_str("10.1.1.1"),
            hostname: String::from("test-host"),
            storage_type: StorageTypeEnum::Ceph,
            array_name: String::from("array-name"),
            pool_name: String::from("unknown"),
        };
        let result = update_storage_info(&info, pid, &configFile).expect(
            "Failed to update
                storage details",
        );

        println!("Successfully updated storage details");
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
    let passed = stmt.query_row(&[&dev_path.to_string_lossy().into_owned()], |row| {
        row.get(0)
    })?;
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
        debug!("Found state: {}", state);

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

#[derive(Debug)]
pub struct HostDetails {
    pub ip: IpAddr,
    pub hostname: String,
    pub region: String,
    pub storage_type: StorageTypeEnum,
    pub array_name: String,
    pub pool_name: String,
}

#[derive(Debug)]
pub enum StorageTypeEnum {
    Ceph,
    Scaleio,
    Gluster,
    Hitachi,
}

impl Display for StorageTypeEnum {
    fn fmt(&self, f: &mut Formatter) -> fResult {
        let message = match *self {
            StorageTypeEnum::Ceph => "ceph",
            StorageTypeEnum::Scaleio => "scaleio",
            StorageTypeEnum::Hitachi => "hitachi",
            StorageTypeEnum::Gluster => "gluster",
        };
        write!(f, "{}", message)
    }
}

#[derive(Debug)]
pub struct DiskInfo {
    pub disk_id: u32,
    pub storage_detail_id: u32,
    pub disk_path: String,
    pub disk_name: String,
    pub disk_uuid: Option<String>
}

impl DiskInfo {
    pub fn new(disk_name: String, disk_path: String, storage_detail_id: u32) -> DiskInfo {
        DiskInfo {
            disk_id: 0,
            disk_name,
            disk_path,
            storage_detail_id,
            disk_uuid: None
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
            storage_detail_id
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

/// Should be called when bynar daemon first starts up
/// Returns whether or not all steps in this call have been successful
/// TODO: return conn, entry_id, region_id, detail_id
pub fn update_storage_info(s_info: &HostDetails, pid: u32, config: &str) -> pResult<bool> {
    debug!("Adding datacenter and host information to database");

    let connection_string = read_db_config(&config);
    if connection_string.is_empty() {
        error!("Failed to build database connection string");
        return Ok(false);
    }
    let conn =
        pConnection::connect(connection_string, TlsMode::None).expect("Database connection failed");
    // extract ip address to a &str
    let ip_address: String = s_info.ip.to_string();
    let entry_id = register_to_process_manager(&conn, pid, &ip_address)?;
    let region_id = update_region(&conn, &s_info.region.clone())?;
    let detail_id = update_storage_details(&conn, &s_info, region_id)?;

    if entry_id == 0 || region_id == 0 || detail_id == 0 {
        error!("Failed to update storage information in the database");
    } else {
        // TODO: Add entry, region_id and storage_id to bynar_operations table
    }
    Ok(true)
}

fn read_db_config(config_file: &str) -> String {
    //TODO: 1. Propagate errors.

    let config_file_fd = match File::open(&config_file) {
        Ok(file) => file,
        Err(_e) => return "".to_string(),
    };
    let config: super::serde_json::Value = match
    super::serde_json::from_reader(config_file_fd) {
        Ok(v) => v,
        Err(_e) => return "".to_string(),
    };

    let mut connection_string = "postgresql://".to_string();
    let username = config["username"].as_str().expect("User name is missing");
    let password = config["password"]
        .as_str()
        .expect("database password is missing");
    let dbname = config["database"]
        .as_str()
        .expect("database name is missing");
    let port = config["port"].as_u64().expect("port number is missing") as u16;
    let endpoint = config["endpoint"]
                                .as_str()
                                .expect("port number is missing");
                                
    //params = ConnectParams::builder()
        //            .user(username, Some(password))
      //              .port(port)
          //          .database(dbname)
            //        .build(Host::Tcp(endpoint.to_string()));
//     usual connection is
//   postgresql://[username[:passwd]@][netloc][:port][,...][/dbname]
    connection_string.push_str(&format!("{}:{}@", username, password));
    connection_string.push_str(endpoint);
    connection_string.push_str(&format!(":{}", port));
    connection_string.push_str(&format!("/{}", dbname));

    connection_string
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
    if let Some(r) = stmt_query.into_iter().next() {
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
            entry_id = r.get(0);
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
            entry_id = r.get(0);
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

    if let Some(r) = stmt_query.into_iter().next() {
        // Exists, return region_id
        region_id = r.get(0);
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
            region_id = res.get(0);
        }
    }
    Ok(region_id)
}

fn update_storage_details(
    conn: &pConnection,
    s_info: &HostDetails,
    region_id: u32,
) -> pResult<u32> {
    let stmt = format!(
        "SELECT storage_id FROM storage_types WHERE storage_type={}",
        s_info.storage_type
    );
    let stmt_query = conn.query(&stmt, &[])?;
    let mut storage_detail_id: u32 = 0;

    if let Some(r) = stmt_query.into_iter().next() {
        let storage_id: u32 = r.get("storage_id");

        // query if these storage details are already in DB
        let details_query = format!(
            "SELECT detail_id FROM storage_details WHERE storage_id = {}
            AND region_id = {} AND hostname = '{}'",
            storage_id, region_id, s_info.hostname
        );
        let details_query_exec = conn.query(&details_query, &[])?;
        if let Some(res) = details_query_exec.into_iter().next() {
            //Exists
            storage_detail_id = res.get("detail_id");
        } else {
            // TODO: modify when exact storage details are added
            let details_query = format!(
                "INSERT INTO storage_details
            (storage_id, region_id, hostname, name_key1) VALUES ({}, {}, '{}',
            '{}' RETURNING detail_id",
                storage_id, region_id, s_info.hostname, s_info.array_name
            );
            let dqr = conn.query(&details_query, &[])?;
            if let Some(result) = dqr.into_iter().next() {
                storage_detail_id = result.get("detail_id");
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
            stmt.push_str("INSERT INTO disks (storage_detail_id, disk_path, disk_name");
            if let Some(ref uuid) = disk_info.disk_uuid {
                stmt.push_str(", disk_uuid");
            }
            stmt.push_str(&format!(") VALUES ({}, {}, {}", disk_info.storage_detail_id,
                            disk_info.disk_path, disk_info.disk_name));
            if let Some(ref uuid) = disk_info.disk_uuid {
                    stmt.push_str(&format!(", {}", uuid));
            }
            
            stmt.push_str(") RETURNING disk_id");
        } 
        _ => {
            // verify if all other details match, select disk_id to match with the
            // return from the insert stmt above
            stmt.push_str(&format!("SELECT disk_id FROM disks WHERE disk_id = {} AND
                                    disk_name = {} AND disk_path = {} AND 
                                    storage_detail_id = {}", disk_info.disk_id,
                                    disk_info.disk_name, disk_info.disk_path,
                                    disk_info.storage_detail_id));
        }
    }
    let stmt_query = conn.query(&stmt, &[])?;

    if let Some(result) = stmt_query
                        .into_iter()
                        .next() {
        disk_info.disk_id = result.get("disk_id");
        Ok(true)
    } else {
        error!("Failed to add disk information to database");
        Ok(false)
    }
}

// inserts the operation record and returns the operation_id
// If updating, performs the query and returns the given operation_id if successful update
pub fn add_or_update_operation(
    conn: &pConnection,
    op_info: &OperationInfo) -> pResult<u32> {
        let mut op_id: u32 = 0;
        let mut stmt = String::new();
        
        match op_info.operation_id {
                    0 => {
                        // no operation_id, validate new record input
                        let host_info: &HostDetailsMapping = &op_info.host_details;
                        if host_info.region_id == 0 ||
                            host_info.entry_id == 0 ||
                                host_info.storage_detail_id == 0 {
                            error!("Required region_id, storage_detail_id, entry_id");
                            return Ok(0);
                        }
                        stmt.push_str("INSERT INTO operations (
                                    region_id, storage_detail_id, entry_id
                                    start_time, disk_id");

                                if let Some(_) = op_info.behalf_of {
                                stmt.push_str(", behalf_of");
                                }
                                if let Some(_) = op_info.reason {
                                stmt.push_str(", reason");
                                }

                                stmt.push_str(")");

                                stmt.push_str(&format!(" VALUES ({},{},{},{}, {}",
                                        host_info.region_id, host_info.storage_detail_id,
                                        host_info.entry_id, op_info.start_time, op_info.disk_id));

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
                        stmt.push_str(&format!("UPDATE operations SET (snapshot_time = {}", op_info.snapshot_time));

                        if let Some(d_time) = op_info.done_time {
                            stmt.push_str(&format!(", done_time = {}", d_time));
                        }
                        stmt.push_str(&format!(") WHERE operation_id = {}", op_info.operation_id));

                        op_id = op_info.operation_id;
                    }
                }
        let stmt_query = conn.query(&stmt, &[])?;
        if let Some(result) = stmt_query.into_iter().next() {
            op_id = result.get("operation_id");
        } else {
            // failed to insert
            error!("Failed to update operation to database");
        }

        Ok(op_id)
    }


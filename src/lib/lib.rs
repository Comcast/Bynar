//! Functions that are needed across most of the workspace.
//!
use serde_derive::*;
use std::fs::read_to_string;
use std::path::Path;

use crate::error::{BynarError, BynarResult};
use api::service::{Disk, Op, OpBoolResult, Operation, ResultType,OpJiraTicketsResult,JiraInfo};
use hashicorp_vault::client::VaultClient;
use log::{debug, error,trace};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use serde::de::DeserializeOwned;
use zmq::{Message, Socket};

pub mod error;
pub mod host_information;

/***** DEMONIZATION CODE *******
mod ffi;

extern crate boxfnonce;
extern crate libc;

use std::env::set_current_dir;
use std::ffi::CString;
use std::fmt;
use std::fs::File;
use std::io;
use std::mem::transmute;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;

use boxfnonce::BoxFnOnce;
use libc::{
    c_int, close, dup2, fork, ftruncate, getpid, open, setgid, setsid, setuid, umask, write,
    LOCK_EX, LOCK_NB,
};
pub use libc::{gid_t, mode_t, uid_t};

use self::ffi::{chroot, flock, get_gid_by_name, get_uid_by_name};

***** DEMONIZATION CODE lib.rs declration  ENDS HERE*******/


pub fn load_config<T>(config_dir: &Path, name: &str) -> BynarResult<T>
where
    T: DeserializeOwned,
{
    let p = config_dir.join(name);
    if !p.exists() {
        error!("{} config file does not exist", p.display());
    }
    let s = read_to_string(p)?;
    let deserialized: T = serde_json::from_str(&s)?;
    Ok(deserialized)
}

pub fn connect(host: &str, port: &str, server_publickey: &str) -> BynarResult<Socket> {
    debug!("Starting zmq sender with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    let client_keypair = zmq::CurveKeyPair::new()?;

    requester.set_curve_serverkey(server_publickey)?;
    requester.set_curve_publickey(&client_keypair.public_key)?;
    requester.set_curve_secretkey(&client_keypair.secret_key)?;
    debug!("Connecting to tcp://{}:{}", host, port);
    assert!(requester
        .connect(&format!("tcp://{}:{}", host, port))
        .is_ok());
    debug!("Client mechanism: {:?}", requester.get_mechanism());

    Ok(requester)
}

pub fn get_vault_token(endpoint: &str, token: &str, hostname: &str) -> BynarResult<String> {
    let client = VaultClient::new(endpoint, token)?;
    let res = client.get_secret(&format!("/{}", hostname))?;
    Ok(res)
}

pub fn add_disk_request(
    s: &mut Socket,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating add disk operation request");
    o.set_Op_type(Op::Add);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if let Some(id) = id {
        o.set_osd_id(id);
    }

    let encoded = o.write_to_bytes().unwrap();
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response");
    let add_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", add_response.len());
    let op_result = parse_from_bytes::<api::service::OpResult>(&add_response)?;
    match op_result.get_result() {
        ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        ResultType::ERR => {
            if op_result.has_error_msg() {
                let msg = op_result.get_error_msg();
                error!("Add disk failed: {}", msg);
                Err(BynarError::from(op_result.get_error_msg()))
            } else {
                error!("Add disk failed but error_msg not set");
                Err(BynarError::from("Add disk failed but error_msg not set"))
            }
        }
    }
}

/*
pub fn check_disk_request(s: &mut Socket) -> Result<RepairResponse, String> {
    let mut o = Operation::new();
    debug!("Creating check disk operation request");
    o.set_Op_type(Op::Check);

    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let check_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", check_response.len());
    let op_result = parse_from_bytes::<api::service::RepairResponse>(&check_response)
        .map_err(|e| e.to_string())?;

    Ok(op_result)
}
*/

pub fn list_disks_request(s: &mut Socket) -> BynarResult<Vec<Disk>> {
    let mut o = Operation::new();
    debug!("Creating list operation request");
    o.set_Op_type(Op::List);

    debug!("Encoding as hex");
    let encoded = o.write_to_bytes()?;
    debug!("{:?}", encoded);

    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response");
    let disks_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", disks_response.len());
    let disk_list = parse_from_bytes::<api::service::Disks>(&disks_response)?;

    let mut d: Vec<Disk> = Vec::new();
    for disk in disk_list.get_disk() {
        d.push(disk.clone());
    }

    Ok(d)
}

pub fn safe_to_remove_request(s: &mut Socket, path: &Path) -> BynarResult<bool> {
    let mut o = Operation::new();
    debug!("Creating safe to remove operation request");
    o.set_Op_type(Op::SafeToRemove);
    o.set_disk(format!("{}", path.display()));
    let encoded = o.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response");
    let safe_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", safe_response.len());
    let op_result = parse_from_bytes::<OpBoolResult>(&safe_response)?;
    match op_result.get_result() {
        ResultType::OK => Ok(op_result.get_value()),
        ResultType::ERR => Err(BynarError::from(op_result.get_error_msg())),
    }
}

pub fn remove_disk_request(
    s: &mut Socket,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating remove operation request");
    o.set_Op_type(Op::Remove);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if id.is_some() {
        o.set_osd_id(id.unwrap());
    }

    let encoded = o.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response");
    let remove_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", remove_response.len());
    let op_result = parse_from_bytes::<api::service::OpResult>(&remove_response)?;
    match op_result.get_result() {
        ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        ResultType::ERR => {
            if op_result.has_error_msg() {
                let msg = op_result.get_error_msg();
                error!("Remove disk failed: {}", msg);
                Err(BynarError::from(op_result.get_error_msg()))
            } else {
                error!("Remove disk failed but error_msg not set");
                Err(BynarError::from("Remove disk failed but error_msg not set"))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSettings {
    pub manager_host: String,
    pub manager_port: u16,
    /// Redfish Ip address or dns name ( Usually iLo where redfish is listening)
    pub redfish_ip: Option<String>,
    /// Redfish credentials
    pub redfish_username: Option<String>,
    /// Redfish credentials
    pub redfish_password: Option<String>,
    /// The port redfish is listening on
    pub redfish_port: Option<u16>,
    pub slack_webhook: Option<String>,
    pub slack_channel: Option<String>,
    pub slack_botname: Option<String>,
    pub vault_endpoint: Option<String>,
    pub vault_token: Option<String>,
    pub jira_user: String,
    pub jira_password: String,
    pub jira_host: String,
    pub jira_issue_type: String,
    pub jira_priority: String,
    pub jira_project_id: String,
    pub jira_ticket_assignee: String,
    pub proxy: Option<String>,
    pub database: DBConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DBConfig {
    pub username: String,
    pub password: Option<String>,
    pub port: u16,
    pub endpoint: String,
    pub dbname: String,
}

pub fn get_jira_tickets(s: &mut Socket) -> BynarResult<()>{
    let mut o = Operation::new();
    debug!("calling get_jira_tickets ");
    o.set_Op_type(Op::GetCreatedTickets);
    let encoded = o.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message in get_jira_tickets");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response: get_jira_tickets");
    let tickets_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", tickets_response.len());
   
    let op_jira_result = parse_from_bytes::<OpJiraTicketsResult>(&tickets_response)?;
    match op_jira_result.get_result() {
        ResultType::OK => {
            debug!("got tickets successfully");
             let proto_jira = op_jira_result.get_tickets();
             let mut jira: Vec<JiraInfo> = Vec::new();
            for JiraInfo in proto_jira {
               debug!("get_ticket_id: {}", JiraInfo.get_ticket_id());
               debug!("get_server_name: {}", JiraInfo.get_server_name());
            }
            Ok(())
        }
        ResultType::ERR => {
            if op_jira_result.has_error_msg() {
                let msg = op_jira_result.get_error_msg();
                error!("get jira tickets failed : {}", msg);
                Err(BynarError::from(op_jira_result.get_error_msg()))
            } else {
                error!("Get jira tickets failed but error_msg not set");
                Err(BynarError::from("Get jira tickets failed but error_msg not set"))
            }
        }
    }
   
}

pub fn set_maintenance(s: &mut Socket) -> BynarResult<()>{
    let mut o = Operation::new();
    debug!("Calling set_maintenance ");
    o.set_Op_type(Op::SetMaintenance);
    let encoded = o.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message in set_maintenance");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response: SetMaintenance");
    let response = s.recv_bytes(0)?;

    Ok(())
    
}

pub fn unset_maintenance(s: &mut Socket) -> BynarResult<()>{
    let mut o = Operation::new();
    debug!("Calling unset_maintenance ");
    o.set_Op_type(Op::UnsetMaintenance);
    let encoded = o.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message in set_maintenance");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response: SetMaintenance");
    let response = s.recv_bytes(0)?;

    Ok(())
    
}


/**** DEMONIZATION CODE *****

macro_rules! tryret {
    ($expr:expr, $ret:expr, $err:expr) => {
        if $expr == -1 {
            return Err($err(errno()));
        } else {
            #[allow(clippy::unused_unit)]
            {
                $ret
            }
        }
    };
}

pub type Errno = c_int;

/// This error type for `Daemonize` `start` method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DaemonizeError {
    /// Unable to fork
    Fork,
    /// Unable to create new session
    DetachSession(Errno),
    /// Unable to resolve group name to group id
    GroupNotFound,
    /// Group option contains NUL
    GroupContainsNul,
    /// Unable to set group
    SetGroup(Errno),
    /// Unable to resolve user name to user id
    UserNotFound,
    /// User option contains NUL
    UserContainsNul,
    /// Unable to set user
    SetUser(Errno),
    /// Unable to change directory
    ChangeDirectory,
    /// pid_file option contains NUL
    PathContainsNul,
    /// Unable to open pid file
    OpenPidfile,
    /// Unable to lock pid file
    LockPidfile(Errno),
    /// Unable to chown pid file
    ChownPidfile(Errno),
    /// Unable to redirect standard streams to /dev/null
    RedirectStreams(Errno),
    /// Unable to write self pid to pid file
    WritePid,
    /// Unable to chroot
    Chroot(Errno),
    // Hints that destructuring should not be exhaustive.
    // This enum may grow additional variants, so this makes sure clients
    // don't count on exhaustive matching. Otherwise, adding a new variant
    // could break existing code.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl DaemonizeError {
    fn __description(&self) -> &str {
        match *self {
            DaemonizeError::Fork => "unable to fork",
            DaemonizeError::DetachSession(_) => "unable to create new session",
            DaemonizeError::GroupNotFound => "unable to resolve group name to group id",
            DaemonizeError::GroupContainsNul => "group option contains NUL",
            DaemonizeError::SetGroup(_) => "unable to set group",
            DaemonizeError::UserNotFound => "unable to resolve user name to user id",
            DaemonizeError::UserContainsNul => "user option contains NUL",
            DaemonizeError::SetUser(_) => "unable to set user",
            DaemonizeError::ChangeDirectory => "unable to change directory",
            DaemonizeError::PathContainsNul => "pid_file option contains NUL",
            DaemonizeError::OpenPidfile => "unable to open pid file",
            DaemonizeError::LockPidfile(_) => "unable to lock pid file",
            DaemonizeError::ChownPidfile(_) => "unable to chown pid file",
            DaemonizeError::RedirectStreams(_) => {
                "unable to redirect standard streams to /dev/null"
            }
            DaemonizeError::WritePid => "unable to write self pid to pid file",
            DaemonizeError::Chroot(_) => "unable to chroot into directory",
            DaemonizeError::__Nonexhaustive => unreachable!(),
        }
    }
}

impl std::fmt::Display for DaemonizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.__description().fmt(f)
    }
}

impl std::error::Error for DaemonizeError {
    fn description(&self) -> &str {
        self.__description()
    }
}

type Result<T> = std::result::Result<T, DaemonizeError>;

/// Expects system user id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum User {
    Name(String),
    Id(uid_t),
}

impl<'a> From<&'a str> for User {
    fn from(t: &'a str) -> User {
        User::Name(t.to_owned())
    }
}

impl From<uid_t> for User {
    fn from(t: uid_t) -> User {
        User::Id(t)
    }
}

/// Expects system group id or name. If name is provided it will be resolved to id later.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Group {
    Name(String),
    Id(gid_t),
}

impl<'a> From<&'a str> for Group {
    fn from(t: &'a str) -> Group {
        Group::Name(t.to_owned())
    }
}

impl From<gid_t> for Group {
    fn from(t: gid_t) -> Group {
        Group::Id(t)
    }
}

#[derive(Debug)]
enum StdioImp {
    Devnull,
    RedirectToFile(File),
}

/// Describes what to do with a standard I/O stream for a child process.
#[derive(Debug)]
pub struct Stdio {
    inner: StdioImp,
}

impl Stdio {
    fn devnull() -> Self {
        Self {
            inner: StdioImp::Devnull,
        }
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Self {
        Self {
            inner: StdioImp::RedirectToFile(file),
        }
    }
}

/// Daemonization options.
///
/// Fork the process in the background, disassociate from its process group and the control terminal.
/// Change umask value to `0o027`, redirect all standard streams to `/dev/null`. Change working
/// directory to `/` or provided value.
///
/// Optionally:
///
///   * maintain and lock the pid-file;
///   * drop user privileges;
///   * drop group privileges;
///   * change root directory;
///   * change the pid-file ownership to provided user (and/or) group;
///   * execute any provided action just before dropping privileges.
///
pub struct Daemonize<T> {
    directory: PathBuf,
    pid_file: Option<PathBuf>,
    chown_pid_file: bool,
    user: Option<User>,
    group: Option<Group>,
    umask: mode_t,
    root: Option<PathBuf>,
    privileged_action: BoxFnOnce<'static, (), T>,
    exit_action: BoxFnOnce<'static, (), ()>,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
}

impl<T> fmt::Debug for Daemonize<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Daemonize")
            .field("directory", &self.directory)
            .field("pid_file", &self.pid_file)
            .field("chown_pid_file", &self.chown_pid_file)
            .field("user", &self.user)
            .field("group", &self.group)
            .field("umask", &self.umask)
            .field("root", &self.root)
            .field("stdin", &self.stdin)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish()
    }
}

impl Daemonize<()> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Daemonize {
            directory: Path::new("/").to_owned(),
            pid_file: None,
            chown_pid_file: false,
            user: None,
            group: None,
            umask: 0o027,
            privileged_action: BoxFnOnce::new(|| ()),
            exit_action: BoxFnOnce::new(|| ()),
            root: None,
            stdin: Stdio::devnull(),
            stdout: Stdio::devnull(),
            stderr: Stdio::devnull(),
        }
    }
}

impl<T> Daemonize<T> {
    /// Create pid-file at `path`, lock it exclusive and write daemon pid.
    pub fn pid_file<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.pid_file = Some(path.as_ref().to_owned());
        self
    }

    /// If `chown` is true, daemonize will change the pid-file ownership, if user or group are provided
    pub fn chown_pid_file(mut self, chown: bool) -> Self {
        self.chown_pid_file = chown;
        self
    }

    /// Change working directory to `path` or `/` by default.
    pub fn working_directory<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.directory = path.as_ref().to_owned();
        self
    }

    /// Drop privileges to `user`.
    pub fn user<U: Into<User>>(mut self, user: U) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Drop privileges to `group`.
    pub fn group<G: Into<Group>>(mut self, group: G) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Change umask to `mask` or `0o027` by default.
    pub fn umask(mut self, mask: mode_t) -> Self {
        self.umask = mask;
        self
    }

    /// Change root to `path`
    pub fn chroot<F: AsRef<Path>>(mut self, path: F) -> Self {
        self.root = Some(path.as_ref().to_owned());
        self
    }

    /// Execute `action` just before dropping privileges. Most common usecase is to open listening socket.
    /// Result of `action` execution will be returned by `start` method.
    pub fn privileged_action<N, F: FnOnce() -> N + 'static>(self, action: F) -> Daemonize<N> {
        let mut new: Daemonize<N> = unsafe { transmute(self) };
        new.privileged_action = BoxFnOnce::new(action);
        new
    }

    /// Execute `action` just before exiting the parent process. Most common usecase is to synchronize with
    /// forked processes.
    pub fn exit_action<F: FnOnce() + 'static>(mut self, action: F) -> Daemonize<T> {
        self.exit_action = BoxFnOnce::new(action);
        self
    }

    /// Configuration for the child process's standard output stream.
    pub fn stdout<S: Into<Stdio>>(mut self, stdio: S) -> Self {
        self.stdout = stdio.into();
        self
    }

    /// Configuration for the child process's standard error stream.
    pub fn stderr<S: Into<Stdio>>(mut self, stdio: S) -> Self {
        self.stderr = stdio.into();
        self
    }

    /// Start daemonization process.
    pub fn start(self) -> std::result::Result<T, DaemonizeError> {
        // Maps an Option<T> to Option<U> by applying a function Fn(T) -> Result<U, DaemonizeError>
        // to a contained value and try! it's result
        macro_rules! maptry {
            ($expr:expr, $f: expr) => {
                match $expr {
                    None => None,
                //    Some(x) => Some(try!($f(x))),
                    Some(x) => Some($f(x)?),
                };
            };
        }

        unsafe {
            let pid_file_fd = maptry!(self.pid_file.clone(), create_pid_file);

            perform_fork(Some(self.exit_action))?;

            set_current_dir(&self.directory).map_err(|_| DaemonizeError::ChangeDirectory)?;
            set_sid()?;
            umask(self.umask);

            perform_fork(None)?;

            redirect_standard_streams(self.stdin, self.stdout, self.stderr)?;

            let uid = maptry!(self.user, get_user);
            let gid = maptry!(self.group, get_group);

            if self.chown_pid_file {
                let args: Option<(PathBuf, uid_t, gid_t)> = match (self.pid_file, uid, gid) {
                    (Some(pid), Some(uid), Some(gid)) => Some((pid, uid, gid)),
                    (Some(pid), None, Some(gid)) => Some((pid, uid_t::max_value() - 1, gid)),
                    (Some(pid), Some(uid), None) => Some((pid, uid, gid_t::max_value() - 1)),
                    // Or pid file is not provided, or both user and group
                    _ => None,
                };

                maptry!(args, |(pid, uid, gid)| chown_pid_file(pid, uid, gid));
            }

            let privileged_action_result = self.privileged_action.call();

            maptry!(self.root, change_root);

            maptry!(gid, set_group);
            maptry!(uid, set_user);

            maptry!(pid_file_fd, write_pid_file);

            Ok(privileged_action_result)
        }
    }
}

unsafe fn perform_fork(exit_action: Option<BoxFnOnce<'static, (), ()>>) -> Result<()> {
    let pid = fork();
    if pid < 0 {
        Err(DaemonizeError::Fork)
    } else if pid == 0 {
        Ok(())
    } else {
        if let Some(exit_action) = exit_action {
            exit_action.call()
        }
        exit(0)
    }
}

unsafe fn set_sid() -> Result<()> {
    tryret!(setsid(), Ok(()), DaemonizeError::DetachSession)
}

unsafe fn redirect_standard_streams(stdin: Stdio, stdout: Stdio, stderr: Stdio) -> Result<()> {
    let devnull_fd = open(b"/dev/null\0" as *const [u8; 10] as _, libc::O_RDWR);
    if -1 == devnull_fd {
        return Err(DaemonizeError::RedirectStreams(errno()));
    }

    let process_stdio = |fd, stdio: Stdio| {
        tryret!(close(fd), (), DaemonizeError::RedirectStreams);
        match stdio.inner {
            StdioImp::Devnull => {
                tryret!(dup2(devnull_fd, fd), (), DaemonizeError::RedirectStreams);
            }
            StdioImp::RedirectToFile(file) => {
                let raw_fd = file.as_raw_fd();
                tryret!(dup2(raw_fd, fd), (), DaemonizeError::RedirectStreams);
            }
        };
        Ok(())
    };

    process_stdio(libc::STDIN_FILENO, stdin)?;
    process_stdio(libc::STDOUT_FILENO, stdout)?;
    process_stdio(libc::STDERR_FILENO, stderr)?;

    tryret!(close(devnull_fd), (), DaemonizeError::RedirectStreams);

    Ok(())
}

unsafe fn get_group(group: Group) -> Result<gid_t> {
    match group {
        Group::Id(id) => Ok(id),
        Group::Name(name) => {
            let s = CString::new(name).map_err(|_| DaemonizeError::GroupContainsNul)?;
            match get_gid_by_name(&s) {
                Some(id) => get_group(Group::Id(id)),
                None => Err(DaemonizeError::GroupNotFound),
            }
        }
    }
}

unsafe fn set_group(group: gid_t) -> Result<()> {
    tryret!(setgid(group), Ok(()), DaemonizeError::SetGroup)
}

unsafe fn get_user(user: User) -> Result<uid_t> {
    match user {
        User::Id(id) => Ok(id),
        User::Name(name) => {
            let s = CString::new(name).map_err(|_| DaemonizeError::UserContainsNul)?;
            match get_uid_by_name(&s) {
                Some(id) => get_user(User::Id(id)),
                None => Err(DaemonizeError::UserNotFound),
            }
        }
    }
}

unsafe fn set_user(user: uid_t) -> Result<()> {
    tryret!(setuid(user), Ok(()), DaemonizeError::SetUser)
}

unsafe fn create_pid_file(path: PathBuf) -> Result<libc::c_int> {
    let path_c = pathbuf_into_cstring(path)?;

    let fd = open(path_c.as_ptr(), libc::O_WRONLY | libc::O_CREAT, 0o666);
    if -1 == fd {
        return Err(DaemonizeError::OpenPidfile);
    }

    tryret!(
        flock(fd, LOCK_EX | LOCK_NB),
        Ok(fd),
        DaemonizeError::LockPidfile
    )
}

unsafe fn chown_pid_file(path: PathBuf, uid: uid_t, gid: gid_t) -> Result<()> {
    let path_c = pathbuf_into_cstring(path)?;
    tryret!(
        libc::chown(path_c.as_ptr(), uid, gid),
        Ok(()),
        DaemonizeError::ChownPidfile
    )
}

unsafe fn write_pid_file(fd: libc::c_int) -> Result<()> {
    let pid = getpid();
    let pid_buf = format!("{}", pid).into_bytes();
    let pid_length = pid_buf.len();
    let pid_c = CString::new(pid_buf).unwrap();
    if -1 == ftruncate(fd, 0) {
        return Err(DaemonizeError::WritePid);
    }
    if write(fd, pid_c.as_ptr() as *const libc::c_void, pid_length) < pid_length as isize {
        Err(DaemonizeError::WritePid)
    } else {
        Ok(())
    }
}

unsafe fn change_root(path: PathBuf) -> Result<()> {
    let path_c = pathbuf_into_cstring(path)?;

    if chroot(path_c.as_ptr()) == 0 {
        Ok(())
    } else {
        Err(DaemonizeError::Chroot(errno()))
    }
}

fn pathbuf_into_cstring(path: PathBuf) -> Result<CString> {
    CString::new(path.into_os_string().into_vec()).map_err(|_| DaemonizeError::PathContainsNul)
}

fn errno() -> Errno {
    io::Error::last_os_error().raw_os_error().expect("errno")
}

**** DEMONIZATION CODE ENDS HERE *****/


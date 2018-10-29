extern crate block_utils;
extern crate ceph;
extern crate ceph_safe_disk;
extern crate goji;
extern crate hashicorp_vault;
extern crate postgres;
extern crate protobuf;
extern crate reqwest;
extern crate rusqlite;
extern crate serde_json;
extern crate slack_hook;
extern crate zmq;

use self::block_utils::BlockUtilsError;
use self::ceph::error::RadosError;
use self::goji::Error as GojiError;
use self::hashicorp_vault::client::error::Error as VaultError;
use self::postgres::Error as PostgresError;
use self::protobuf::ProtobufError;
use self::reqwest::Error as ReqwestError;
use self::rusqlite::Error as SqliteError;
use self::serde_json::Error as SerdeJsonError;
use self::slack_hook::Error as SlackError;
use self::zmq::Error as ZmqError;
use std::error::Error as err;
use std::fmt;
use std::io::Error as IOError;
use std::num::ParseIntError;

pub type BynarResult<T> = Result<T, BynarError>;

/// Custom error handling
#[derive(Debug)]
pub enum BynarError {
    Error(String),
    GojiError(GojiError),
    IoError(IOError),
    ParseIntError(ParseIntError),
    ProtobufError(ProtobufError),
    RadosError(RadosError),
    ReqwestError(ReqwestError),
    SerdeJsonError(SerdeJsonError),
    SlackError(SlackError),
    SqliteError(SqliteError),
    VaultError(VaultError),
    ZmqError(ZmqError),
    BlockUtilsError(BlockUtilsError),
    PostgresError(PostgresError),
}

impl fmt::Display for BynarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl err for BynarError {
    fn description(&self) -> &str {
        match *self {
            BynarError::Error(ref e) => &e,
            BynarError::GojiError(ref e) => e.description(),
            BynarError::IoError(ref e) => e.description(),
            BynarError::ParseIntError(ref e) => e.description(),
            BynarError::ProtobufError(ref e) => e.description(),
            BynarError::RadosError(ref e) => e.description(),
            BynarError::ReqwestError(ref e) => e.description(),
            BynarError::SerdeJsonError(ref e) => e.description(),
            BynarError::SlackError(ref e) => e.description(),
            BynarError::SqliteError(ref e) => e.description(),
            BynarError::VaultError(ref e) => e.description(),
            BynarError::ZmqError(ref e) => e.description(),
            BynarError::BlockUtilsError(ref e) => e.description(),
            BynarError::PostgresError(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&err> {
        match *self {
            BynarError::Error(_) => None,
            BynarError::GojiError(ref e) => e.cause(),
            BynarError::IoError(ref e) => e.cause(),
            BynarError::ParseIntError(ref e) => e.cause(),
            BynarError::ProtobufError(ref e) => e.cause(),
            BynarError::RadosError(ref e) => e.cause(),
            BynarError::ReqwestError(ref e) => e.cause(),
            BynarError::SerdeJsonError(ref e) => e.cause(),
            BynarError::SlackError(ref e) => e.cause(),
            BynarError::SqliteError(ref e) => e.cause(),
            BynarError::VaultError(ref e) => e.cause(),
            BynarError::ZmqError(ref e) => e.cause(),
            BynarError::BlockUtilsError(ref e) => e.cause(),
            BynarError::PostgresError(ref e) => e.cause(),
        }
    }
}

impl BynarError {
    /// Create a new BynarError with a String message
    pub fn new(err: String) -> BynarError {
        BynarError::Error(err)
    }

    /// Convert a BynarError into a String representation.
    pub fn to_string(&self) -> String {
        match *self {
            BynarError::Error(ref err) => err.to_string(),
            BynarError::GojiError(ref err) => err.to_string(),
            BynarError::IoError(ref err) => err.to_string(),
            BynarError::ParseIntError(ref err) => err.to_string(),
            BynarError::ProtobufError(ref err) => err.to_string(),
            BynarError::RadosError(ref err) => err.to_string(),
            BynarError::ReqwestError(ref err) => err.to_string(),
            BynarError::SerdeJsonError(ref err) => err.to_string(),
            BynarError::SlackError(ref err) => err.to_string(),
            BynarError::SqliteError(ref err) => err.to_string(),
            BynarError::VaultError(ref err) => err.to_string(),
            BynarError::ZmqError(ref err) => err.to_string(),
            BynarError::BlockUtilsError(ref err) => err.to_string(),
            BynarError::PostgresError(ref err) => err.to_string(),
        }
    }
}

impl From<GojiError> for BynarError {
    fn from(err: GojiError) -> BynarError {
        BynarError::GojiError(err)
    }
}

impl From<IOError> for BynarError {
    fn from(err: IOError) -> BynarError {
        BynarError::IoError(err)
    }
}

impl From<ParseIntError> for BynarError {
    fn from(err: ParseIntError) -> BynarError {
        BynarError::ParseIntError(err)
    }
}

impl From<ProtobufError> for BynarError {
    fn from(err: ProtobufError) -> BynarError {
        BynarError::ProtobufError(err)
    }
}

impl From<RadosError> for BynarError {
    fn from(err: RadosError) -> BynarError {
        BynarError::RadosError(err)
    }
}

impl From<ReqwestError> for BynarError {
    fn from(err: ReqwestError) -> BynarError {
        BynarError::ReqwestError(err)
    }
}

impl From<SerdeJsonError> for BynarError {
    fn from(err: SerdeJsonError) -> BynarError {
        BynarError::SerdeJsonError(err)
    }
}

impl From<SlackError> for BynarError {
    fn from(err: SlackError) -> BynarError {
        BynarError::SlackError(err)
    }
}

impl From<SqliteError> for BynarError {
    fn from(err: SqliteError) -> BynarError {
        BynarError::SqliteError(err)
    }
}

impl From<String> for BynarError {
    fn from(err: String) -> BynarError {
        BynarError::new(err)
    }
}

impl From<VaultError> for BynarError {
    fn from(err: VaultError) -> BynarError {
        BynarError::VaultError(err)
    }
}

impl From<ZmqError> for BynarError {
    fn from(err: ZmqError) -> BynarError {
        BynarError::ZmqError(err)
    }
}
impl From<BlockUtilsError> for BynarError {
    fn from(err: BlockUtilsError) -> BynarError {
        BynarError::BlockUtilsError(err)
    }
}
impl From<PostgresError> for BynarError {
    fn from(err: PostgresError) -> BynarError {
        BynarError::PostgresError(err)
    }
}

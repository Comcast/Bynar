use blkid::BlkidError;
use block_utils::BlockUtilsError;
use ceph::error::RadosError;
use goji::Error as GojiError;
use hashicorp_vault::client::error::Error as VaultError;
use lvm::LvmError;
use nix::Error as NixError;
use postgres::Error as PostgresError;
use protobuf::ProtobufError;
use pwd::PwdError;
use r2d2::Error as R2d2Error;
use reqwest::Error as ReqwestError;
use rusqlite::Error as SqliteError;
use serde_json::Error as SerdeJsonError;
use slack_hook::Error as SlackError;
use uuid::parser::ParseError as UuidError;
use zmq::Error as ZmqError;

use std::error::Error as err;
use std::fmt;
use std::io::Error as IOError;
use std::num::ParseIntError;

pub type BynarResult<T> = Result<T, BynarError>;

/// Custom error handling
#[derive(Debug)]
pub enum BynarError {
    BlkidError(BlkidError),
    BlockUtilsError(BlockUtilsError),
    Error(String),
    GojiError(GojiError),
    HardwareError {
        error: String,
        name: String,
        location: Option<String>,
        location_format: Option<String>,
        serial_number: Option<String>,
    },
    IoError(IOError),
    LvmError(LvmError),
    NixError(NixError),
    ParseIntError(ParseIntError),
    PostgresError(PostgresError),
    ProtobufError(ProtobufError),
    PwdError(PwdError),
    R2d2Error(R2d2Error),
    RadosError(RadosError),
    ReqwestError(ReqwestError),
    SerdeJsonError(SerdeJsonError),
    SlackError(SlackError),
    SqliteError(SqliteError),
    UuidError(UuidError),
    VaultError(VaultError),
    ZmqError(ZmqError),
}

impl fmt::Display for BynarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BynarError::BlkidError(ref e) => write!(f, "{}", e),
            BynarError::BlockUtilsError(ref e) => write!(f, "{}", e.description()),
            BynarError::Error(ref e) => write!(f, "{}", e),
            BynarError::GojiError(ref e) => write!(f, "{}", e),
            BynarError::HardwareError {
                ref name,
                ref location,
                ref location_format,
                ref error,
                ref serial_number,
            } => {
                let mut err = format!("Error: {}. {}", error, name);
                if let Some(serial) = serial_number {
                    err.push_str(&format!(" with serial {}", serial));
                }
                if let Some(lo) = location {
                    err.push_str(&format!(" at location {} ", lo));
                }
                if let Some(lo_fmt) = location_format {
                    err.push_str(&format!(" with format {} ", lo_fmt));
                }
                write!(f, "{}", err)
            }
            BynarError::IoError(ref e) => write!(f, "{}", e),
            BynarError::LvmError(ref e) => write!(f, "{}", e),
            BynarError::NixError(ref e) => write!(f, "{}", e),
            BynarError::ParseIntError(ref e) => write!(f, "{}", e),
            BynarError::PostgresError(ref e) => write!(f, "{}", e),
            BynarError::ProtobufError(ref e) => write!(f, "{}", e),
            BynarError::PwdError(ref e) => match e {
                PwdError::StringConvError(s) => write!(f, "{}", s),
                PwdError::NullPtr => write!(f, "Null pointer err"),
            },
            BynarError::R2d2Error(ref e) => write!(f, "{}", e),
            BynarError::RadosError(ref e) => write!(f, "{}", e),
            BynarError::ReqwestError(ref e) => write!(f, "{}", e),
            BynarError::SerdeJsonError(ref e) => write!(f, "{}", e),
            BynarError::SlackError(ref e) => write!(f, "{}", e),
            BynarError::SqliteError(ref e) => write!(f, "{}", e),
            BynarError::UuidError(ref e) => write!(f, "{}", e),
            BynarError::VaultError(ref e) => write!(f, "{}", e),
            BynarError::ZmqError(ref e) => write!(f, "{}", e),
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
            BynarError::BlkidError(ref err) => err.to_string(),
            BynarError::BlockUtilsError(ref err) => err.to_string(),
            BynarError::Error(ref err) => err.to_string(),
            BynarError::GojiError(ref err) => err.to_string(),
            BynarError::HardwareError {
                ref name,
                ref location,
                ref location_format,
                ref error,
                ref serial_number,
            } => {
                let mut err = format!("Error: {}. {}", error, name);
                if let Some(serial) = serial_number {
                    err.push_str(&format!(" with serial {}", serial));
                }
                if let Some(lo) = location {
                    err.push_str(&format!(" at location {} ", lo));
                }
                if let Some(lo_fmt) = location_format {
                    err.push_str(&format!(" with format {} ", lo_fmt));
                }
                err
            }
            BynarError::IoError(ref err) => err.to_string(),
            BynarError::LvmError(ref err) => err.to_string(),
            BynarError::NixError(ref err) => err.to_string(),
            BynarError::ParseIntError(ref err) => err.to_string(),
            BynarError::PostgresError(ref err) => err.to_string(),
            BynarError::ProtobufError(ref err) => err.to_string(),
            BynarError::PwdError(ref err) => err.to_string(),
            BynarError::R2d2Error(ref e) => e.to_string(),
            BynarError::RadosError(ref err) => err.to_string(),
            BynarError::ReqwestError(ref err) => err.to_string(),
            BynarError::SerdeJsonError(ref err) => err.to_string(),
            BynarError::SlackError(ref err) => err.to_string(),
            BynarError::SqliteError(ref err) => err.to_string(),
            BynarError::UuidError(ref err) => err.to_string(),
            BynarError::VaultError(ref err) => err.to_string(),
            BynarError::ZmqError(ref err) => err.to_string(),
        }
    }
}

impl From<BlkidError> for BynarError {
    fn from(err: BlkidError) -> BynarError {
        BynarError::BlkidError(err)
    }
}

impl From<BlockUtilsError> for BynarError {
    fn from(err: BlockUtilsError) -> BynarError {
        BynarError::BlockUtilsError(err)
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

impl From<LvmError> for BynarError {
    fn from(err: LvmError) -> BynarError {
        BynarError::LvmError(err)
    }
}

impl From<NixError> for BynarError {
    fn from(err: NixError) -> BynarError {
        BynarError::NixError(err)
    }
}

impl From<ParseIntError> for BynarError {
    fn from(err: ParseIntError) -> BynarError {
        BynarError::ParseIntError(err)
    }
}

impl From<PostgresError> for BynarError {
    fn from(err: PostgresError) -> BynarError {
        BynarError::PostgresError(err)
    }
}

impl From<ProtobufError> for BynarError {
    fn from(err: ProtobufError) -> BynarError {
        BynarError::ProtobufError(err)
    }
}

impl From<PwdError> for BynarError {
    fn from(err: PwdError) -> BynarError {
        BynarError::PwdError(err)
    }
}

impl From<R2d2Error> for BynarError {
    fn from(err: R2d2Error) -> BynarError {
        BynarError::R2d2Error(err)
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

impl<'a> From<&'a str> for BynarError {
    fn from(err: &str) -> BynarError {
        BynarError::new(err.to_string())
    }
}

impl From<String> for BynarError {
    fn from(err: String) -> BynarError {
        BynarError::new(err)
    }
}

impl From<UuidError> for BynarError {
    fn from(err: UuidError) -> BynarError {
        BynarError::UuidError(err)
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

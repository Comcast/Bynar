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
    BlockUtilsError(BlockUtilsError),
    Error(String),
    GojiError(GojiError),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl err for BynarError {
    fn description(&self) -> &str {
        match *self {
            BynarError::BlockUtilsError(ref e) => e.description(),
            BynarError::Error(ref e) => &e,
            BynarError::GojiError(ref e) => e.description(),
            BynarError::IoError(ref e) => e.description(),
            BynarError::LvmError(ref e) => e.description(),
            BynarError::NixError(ref e) => e.description(),
            BynarError::ParseIntError(ref e) => e.description(),
            BynarError::PostgresError(ref e) => e.description(),
            BynarError::ProtobufError(ref e) => e.description(),
            BynarError::PwdError(ref e) => match e {
                PwdError::StringConvError(s) => &s,
                PwdError::NullPtr => "nullptr",
            },
            BynarError::R2d2Error(ref e) => e.description(),
            BynarError::RadosError(ref e) => e.description(),
            BynarError::ReqwestError(ref e) => e.description(),
            BynarError::SerdeJsonError(ref e) => e.description(),
            BynarError::SlackError(ref e) => e.description(),
            BynarError::SqliteError(ref e) => e.description(),
            BynarError::UuidError(ref e) => e.description(),
            BynarError::VaultError(ref e) => e.description(),
            BynarError::ZmqError(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&err> {
        match *self {
            BynarError::BlockUtilsError(ref e) => e.cause(),
            BynarError::Error(_) => None,
            BynarError::GojiError(ref e) => e.cause(),
            BynarError::IoError(ref e) => e.cause(),
            BynarError::LvmError(ref e) => e.cause(),
            BynarError::NixError(ref e) => e.cause(),
            BynarError::ParseIntError(ref e) => e.cause(),
            BynarError::PostgresError(ref e) => e.cause(),
            BynarError::ProtobufError(ref e) => e.cause(),
            BynarError::PwdError(_) => None,
            BynarError::R2d2Error(ref e) => e.cause(),
            BynarError::RadosError(ref e) => e.cause(),
            BynarError::ReqwestError(ref e) => e.cause(),
            BynarError::SerdeJsonError(ref e) => e.cause(),
            BynarError::SlackError(ref e) => e.cause(),
            BynarError::SqliteError(ref e) => e.cause(),
            BynarError::UuidError(ref e) => e.cause(),
            BynarError::VaultError(ref e) => e.cause(),
            BynarError::ZmqError(ref e) => e.cause(),
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
            BynarError::BlockUtilsError(ref err) => err.to_string(),
            BynarError::Error(ref err) => err.to_string(),
            BynarError::GojiError(ref err) => err.to_string(),
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

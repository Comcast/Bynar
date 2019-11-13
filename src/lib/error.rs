use blkid::BlkidError;
use block_utils::BlockUtilsError;
use ceph::error::RadosError;
use derive_error as de;
use goji::Error as GojiError;
use hashicorp_vault::client::error::Error as VaultError;
use lvm::LvmError;
use nix::Error as NixError;
use postgres::Error as PostgresError;
use protobuf::ProtobufError;
use pwd::PwdError;
use r2d2::Error as R2d2Error;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use slack_hook::Error as SlackError;
use uuid::parser::ParseError as UuidError;
use zmq::Error as ZmqError;

use std::fmt;
use std::io::Error as IOError;
use std::num::ParseIntError;

pub type BynarResult<T> = Result<T, BynarError>;

#[derive(Debug)]
pub struct HardwareError {
    pub error: String,
    pub name: String,
    pub location: Option<String>,
    pub location_format: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Debug)]
pub enum PwdBError {
    PwdError(PwdError),
}

impl fmt::Display for PwdBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PwdBError::PwdError(PwdError::StringConvError(s)) => write!(f, "{}", s),
            PwdBError::PwdError(PwdError::NullPtr) => write!(f, "Null pointer err"),
        }
    }
}

impl fmt::Display for HardwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut err = format!("Error: {}. {}", self.error, self.name);
        if let Some(serial) = &self.serial_number {
            err.push_str(&format!(" with serial {}", serial));
        }
        if let Some(lo) = &self.location {
            err.push_str(&format!(" at location {} ", lo));
        }
        if let Some(lo_fmt) = &self.location_format {
            err.push_str(&format!(" with format {} ", lo_fmt));
        }
        write!(f, "{}", err)
    }
}

/// Custom error handling
#[derive(Debug, de::Error)]
pub enum BynarError {
    BlkidError(BlkidError),
    BlockUtilsError(BlockUtilsError),
    #[error(msg_embedded, non_std, no_from)]
    Error(String),
    GojiError(GojiError),
    #[error(msg, non_std, no_from)]
    HardwareError(HardwareError),
    IoError(IOError),
    LvmError(LvmError),
    NixError(NixError),
    ParseIntError(ParseIntError),
    PostgresError(PostgresError),
    ProtobufError(ProtobufError),
    #[error(msg, non_std, no_from)]
    PwdError(PwdBError),
    R2d2Error(R2d2Error),
    #[error(msg, non_std)]
    RadosError(RadosError),
    ReqwestError(ReqwestError),
    SerdeJsonError(SerdeJsonError),
    SlackError(SlackError),
    UuidError(UuidError),
    VaultError(VaultError),
    ZmqError(ZmqError),
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
            BynarError::HardwareError(HardwareError {
                ref name,
                ref location,
                ref location_format,
                ref error,
                ref serial_number,
            }) => {
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
            BynarError::UuidError(ref err) => err.to_string(),
            BynarError::VaultError(ref err) => err.to_string(),
            BynarError::ZmqError(ref err) => err.to_string(),
        }
    }
}
impl From<PwdError> for BynarError {
    fn from(err: PwdError) -> BynarError {
        BynarError::PwdError(PwdBError::PwdError(err))
    }
}

impl From<String> for BynarError {
    fn from(err: String) -> BynarError {
        BynarError::new(err)
    }
}

impl<'a> From<&'a str> for BynarError {
    fn from(err: &str) -> BynarError {
        BynarError::new(err.to_string())
    }
}

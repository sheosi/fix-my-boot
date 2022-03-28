use std::io;
use std::process::{ExitStatus, Output};
use std::str::FromStr;

use num_traits::FromPrimitive;
use num_derive::FromPrimitive;
use thiserror::Error;

pub enum Distribution {
    Fedora,
    Unknown
}

impl Distribution {
    pub fn loader_name(&self) -> &'static str {
        match self {
            Distribution::Fedora => "Fedora",
            Distribution::Unknown => "Linux Loader"
        }
    }
}

impl ToString for Distribution {
    fn to_string(&self) -> String {
        match self {
            Distribution::Fedora => "Fedora".to_string(),
            Distribution::Unknown => "Linux".to_string()
        }
    }
}

impl FromStr for Distribution {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fedora" => Ok(Distribution::Fedora),
            _ => Err(())
        }
    }
}

#[derive(Error, Debug)]
pub enum CallError {
    #[error("The program had an error")]
    ProgramError,
    #[error("The program was not found")]
    NotFound,
    #[error("The user has not enough permissions for this program")]
    PermissionDenied,
    #[error("Other error")]
    OtherError,
}

impl CallError {
    pub fn from_res(res: Result<ExitStatus, io::Error>) -> Result<(),Self> {
        match res {
            Ok(status) => {
                match status.code() {
                    Some(0) => Ok(()),
                    _ => Err(CallError::ProgramError)
                }
            },
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => Err(CallError::NotFound),
                io::ErrorKind::PermissionDenied => Err(CallError::ProgramError),
                _ => Err(CallError::OtherError)
            }
        }
    }

    pub fn from_output(res: Result<Output, io::Error>)-> Result<Output, Self> {
        Ok(res.unwrap())
    }
}

#[derive(Error, Debug, FromPrimitive)]
pub enum ReinstallError {
    #[error("The program had an error")]
    ProgramError = 166,
    #[error("The program was not found")]
    NotFound,
    #[error("The user has not enough permissions for this program")]
    PermissionDenied,
    #[error("Other error")]
    OtherError
}

impl ReinstallError {
    pub fn from_res(res: ExitStatus) -> Result<(), Self> {
        match res.code() {
            Some(0) => Ok(()),
            Some(code) => Err(FromPrimitive::from_i32(code).unwrap()),
            None => Err(ReinstallError::OtherError)
        }
    }
}

impl From<CallError> for ReinstallError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::ProgramError => ReinstallError::ProgramError,
            CallError::NotFound => ReinstallError::NotFound,
            CallError::PermissionDenied => ReinstallError::PermissionDenied,
            CallError::OtherError => ReinstallError::OtherError
        }
    }
}
use log::error;

#[derive(Debug)]
pub enum AlpacaError {
    PLPL(ephemeris::PLPLError),
    Apca(apca::Error),
    Logger(log::SetLoggerError),
    Io(std::io::Error),
}

impl std::fmt::Display for AlpacaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlpacaError::PLPL(e) => {
                error!("PLPL error: {:?}", e);
                write!(f, "PLPL error: {:?}", e)
            }
            AlpacaError::Apca(e) => {
                error!("Apca error: {:?}", e);
                write!(f, "Apca error: {:?}", e)
            }
            AlpacaError::Logger(e) => {
                error!("Logger error: {:?}", e);
                write!(f, "Logger error: {:?}", e)
            }
            AlpacaError::Io(e) => {
                error!("IO error: {:?}", e);
                write!(f, "IO error: {:?}", e)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AlpacaError>;

impl From<ephemeris::PLPLError> for AlpacaError {
    fn from(e: ephemeris::PLPLError) -> Self {
        AlpacaError::PLPL(e)
    }
}

impl From<apca::Error> for AlpacaError {
    fn from(e: apca::Error) -> Self {
        AlpacaError::Apca(e)
    }
}

impl From<log::SetLoggerError> for AlpacaError {
    fn from(e: log::SetLoggerError) -> Self {
        AlpacaError::Logger(e)
    }
}

impl From<std::io::Error> for AlpacaError {
    fn from(e: std::io::Error) -> Self {
        AlpacaError::Io(e)
    }
}

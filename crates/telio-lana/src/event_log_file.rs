//! This module is a mock for libmoose API, it logs moose function calls to file.
//!
use std::{fs::File, fs::OpenOptions, io::Write};
use time;

pub use telio_utils::telio_log_warn;

const LOGFILE_PATH: &str = "events-moose.log";

/// Mock for setting the device info on libmoose.
/// Is logging every piece of info helpful or a general log would be enough?
#[allow(unused_must_use)]
pub fn init_device_info() {
    let foreign_tracker = "nordvpnapp";

    moose::fetch_specific_context(foreign_tracker);
    event_log("set_context_device_brand", Some(vec!["NA"]));
    event_log("set_context_device_fp", Some(vec!["NA"]));
    event_log("set_context_device_location_city", Some(vec!["NA"]));
    event_log("set_context_device_location_country", Some(vec!["NA"]));
    event_log("set_context_device_location_region", Some(vec!["NA"]));
    event_log("set_context_device_model", Some(vec!["NA"]));
    event_log("set_context_device_os", Some(vec!["NA"]));
    event_log("set_context_device_resolution", Some(vec!["NA"]));
    event_log("set_context_device_timeZone", Some(vec!["NA"]));
    event_log("set_context_device_type", Some(vec!["NA"]));
}

/// Logs a function call and its arguments to file.
///
/// Parameters:
/// * func_name - Name of the called function.
/// * arg_set   - Contains the arguments passed to the called function, if any.
/// Returns:
/// * The number of bytes written into file or moose::Error otherwise
fn event_log(
    func_name: &str,
    arg_set: Option<Vec<&str>>,
) -> std::result::Result<usize, moose::Error> {
    let file = if std::path::Path::new(&LOGFILE_PATH).exists() {
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(LOGFILE_PATH)
    } else {
        File::create(LOGFILE_PATH)
    };

    if let Ok(mut f) = file {
        let format_string =
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second] ")
                .map_err(|_| moose::Error::EventLogError)?;
        let mut buffer = time::OffsetDateTime::now_utc()
            .format(&format_string)
            .map_err(|_| moose::Error::EventLogError)?;
        buffer.push_str(func_name);
        buffer.push('(');
        if let Some(args) = arg_set {
            let mut it = args.iter().peekable();
            while let Some(arg) = it.next() {
                match it.peek() {
                    Some(_) => buffer.push_str(format!("{}, ", arg).as_str()),
                    None => buffer.push_str(arg),
                }
            }
        }
        buffer.push_str(")\n");

        Ok(f.write(buffer.as_bytes()).unwrap_or(0))
    } else {
        Err(moose::Error::EventLogError)
    }
}

#[allow(missing_docs)]
/// Module that mocks moose items.
pub mod moose {
    use serde::{Deserialize, Serialize};

    /// Mock of moose::MeshnetappContext.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct MeshnetappContext {
        #[serde(rename = "application")]
        pub application: MeshnetappContextApplication,
    }

    /// Mock of moose::MeshnetappContextApplicationConfig.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct MeshnetappContextApplicationConfig {
        #[serde(rename = "current_state")]
        pub current_state: MeshnetappContextApplicationConfigCurrentState,
    }

    /// Mock of moose::MeshnetappContextApplicationConfigCurrentState.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct MeshnetappContextApplicationConfigCurrentState {
        #[serde(rename = "internal_meshnet")]
        pub internal_meshnet: MeshnetappContextApplicationConfigCurrentStateInternalMeshnet,
    }

    /// Mock of moose::MeshnetappContextApplicationConfigCurrentStateInternalMeshnet.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct MeshnetappContextApplicationConfigCurrentStateInternalMeshnet {
        #[serde(rename = "fp")]
        pub fp: Option<String>,
    }

    /// Mock of moose::MeshnetappContextApplication.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct MeshnetappContextApplication {
        #[serde(rename = "config")]
        pub config: MeshnetappContextApplicationConfig,
    }

    /// Logger error
    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("Logger was not initiated")]
        NotInitiatedError,
        #[error("Failed to log event")]
        EventLogError,
    }

    /// Logger result
    #[derive(Debug, PartialEq)]
    pub enum Result {
        Success,
        AlreadyInitiated,
    }

    /// Initialize logger file with current date and time.
    ///
    /// Parameters:
    /// * event_path    - path of the DB file where events would be stored.
    /// * app_name      - Lana's application name
    /// * app_version   - Semantic version of the application.
    /// * exp_moose_ver - Eventual moose version
    /// * prod          - wether the events should be sent to production or not
    #[allow(unused_variables)]
    pub fn init(
        event_path: String,
        app_name: String,
        app_version: String,
        exp_moose_ver: String,
        prod: bool,
    ) -> std::result::Result<Result, Error> {
        match super::event_log(
            "init",
            Some(vec![
                event_path.as_str(),
                app_name.as_str(),
                app_version.as_str(),
                exp_moose_ver.as_str(),
                prod.to_string().as_str(),
            ]),
        ) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    pub fn moose_deinit() -> std::result::Result<Result, Error> {
        match super::event_log("moose_deinit", None) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    #[allow(non_snake_case)]
    /// Mocked moose function.
    pub fn set_context_application_config_currentState_externalLinks(
        val: String,
    ) -> std::result::Result<Result, Error> {
        match super::event_log(
            "set_context_application_config_currentState_externalLinks",
            Some(vec![val.as_str()]),
        ) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    #[allow(non_snake_case)]
    /// Mocked moose function.
    pub fn set_context_application_config_currentState_internalMeshnet_fp(
        val: String,
    ) -> std::result::Result<Result, Error> {
        match super::event_log(
            "set_context_application_config_currentState_internalMeshnet_fp",
            Some(vec![val.as_str()]),
        ) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    #[allow(non_snake_case)]
    /// Mocked moose function.
    pub fn set_context_application_config_currentState_internalMeshnet_members(
        val: String,
    ) -> std::result::Result<Result, Error> {
        match super::event_log(
            "set_context_application_config_currentState_internalMeshnet_members",
            Some(vec![val.as_str()]),
        ) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    #[allow(non_snake_case)]
    /// Mocked moose function.
    pub fn set_context_application_config_currentState_internalMeshnet_connectivityMatrix(
        val: String,
    ) -> std::result::Result<Result, Error> {
        match super::event_log(
            "set_context_application_config_currentState_internalMeshnet_connectivityMatrix",
            Some(vec![val.as_str()]),
        ) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    #[allow(non_snake_case)]
    /// Mocked moose function.
    pub fn send_serviceQuality_node_heartbeat(
        connectionDuration: String,
        heartbeatInterval: i32,
        receivedData: String,
        rtt: String,
        sentData: String,
    ) -> std::result::Result<Result, Error> {
        let heartbeatIntervalString = heartbeatInterval.to_string();
        let args = vec![
            connectionDuration.as_str(),
            heartbeatIntervalString.as_str(),
            receivedData.as_str(),
            rtt.as_str(),
            sentData.as_str(),
        ];

        match super::event_log("send_serviceQuality_node_heartbeat", Some(args)) {
            Ok(_) => Ok(Result::Success),
            _ => Err(Error::EventLogError),
        }
    }

    /// Mocked moose function.
    pub fn fetch_context() -> std::result::Result<MeshnetappContext, Error> {
        match super::event_log("fetch_context", None) {
            Ok(_) => Ok(empty_context()),
            _ => Err(Error::EventLogError),
        }
    }

    /// Mocked moose function.
    fn empty_context() -> MeshnetappContext {
        MeshnetappContext {
            application: MeshnetappContextApplication {
                config: MeshnetappContextApplicationConfig {
                    current_state: MeshnetappContextApplicationConfigCurrentState {
                        internal_meshnet:
                            MeshnetappContextApplicationConfigCurrentStateInternalMeshnet {
                                fp: None,
                            },
                    },
                },
            },
        }
    }

    /// Mocked moose function.
    pub fn fetch_specific_context(name: &str) -> std::result::Result<String, Error> {
        match super::event_log("fetch_specific_context", Some(vec![name])) {
            Ok(_) => Ok(String::from("MockedDevice")),
            _ => Err(Error::EventLogError),
        }
    }
}

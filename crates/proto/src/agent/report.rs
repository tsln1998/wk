use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Events {
    EvtMachineEmit(EvtMachineEmit),
    EvtOsEmit(EvtOsEmit),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EvtMachineEmit {
    pub ip: String,
    pub country: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EvtOsEmit {
    pub family: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
    pub build: Option<String>,
    pub virtualization: Option<bool>,
}

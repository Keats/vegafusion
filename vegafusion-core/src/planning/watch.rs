use std::convert::TryFrom;
use itertools::Itertools;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::error::VegaFusionError;
use crate::planning::stitch::CommPlan;
use crate::proto::gen::tasks::{Variable, VariableNamespace};
use crate::error::Result;
use crate::task_graph::task_graph::ScopedVariable;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchNamespace {
    Signal,
    Data,
}

impl TryFrom<VariableNamespace> for WatchNamespace {
    type Error = VegaFusionError;

    fn try_from<>(value: VariableNamespace<>) -> Result<Self> {
        match value {
            VariableNamespace::Signal => Ok(Self::Signal),
            VariableNamespace::Data => Ok(Self::Data),
            _ => Err(VegaFusionError::internal("Scale namespace not supported")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Watch {
    pub namespace: WatchNamespace,
    pub name: String,
    pub scope: Vec<u32>,
}

impl Watch {
    pub fn to_scoped_variable(&self) -> ScopedVariable {
        (
            match self.namespace {
                WatchNamespace::Signal => Variable::new_signal(&self.name),
                WatchNamespace::Data => Variable::new_data(&self.name),
            },
            self.scope.clone(),
        )
    }
}

impl TryFrom<ScopedVariable> for Watch {
    type Error = VegaFusionError;

    fn try_from(value: ScopedVariable) -> Result<Self> {
        let tmp = value.0.namespace();
        let tmp = WatchNamespace::try_from(tmp)?;
        Ok(Self {
            namespace: tmp,
            name: value.0.name.clone(),
            scope: value.1,
        })
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPlan {
    pub server_to_client: Vec<Watch>,
    pub client_to_server: Vec<Watch>,
}

impl From<CommPlan> for WatchPlan {
    fn from(value: CommPlan) -> Self {
        Self {
            server_to_client: value
                .server_to_client
                .into_iter()
                .map(|scoped_var| Watch::try_from(scoped_var).unwrap())
                .sorted()
                .collect(),
            client_to_server: value
                .client_to_server
                .into_iter()
                .map(|scoped_var| Watch::try_from(scoped_var).unwrap())
                .sorted()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchValue {
    pub watch: Watch,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchValues {
    pub values: Vec<WatchValue>,
}
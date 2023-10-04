use fedimint_client::sm::{DynState, OperationId, State, StateTransition};

use fedimint_client::DynGlobalClientContext;

use fedimint_core::core::{IntoDynInstance, ModuleInstanceId};

use fedimint_core::encoding::{Decodable, Encodable};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::NostimintClientContext;

/// Tracks a transaction
#[derive(Debug, Clone, Eq, PartialEq, Decodable, Encodable)]
pub enum NostimintStateMachine {}

impl State for NostimintStateMachine {
    type ModuleContext = NostimintClientContext;
    type GlobalContext = DynGlobalClientContext;

    fn transitions(
        &self,
        _context: &Self::ModuleContext,
        _global_context: &Self::GlobalContext,
    ) -> Vec<StateTransition<Self>> {
        todo!()
    }

    fn operation_id(&self) -> OperationId {
        todo!()
    }
}

// TODO: Boiler-plate
impl IntoDynInstance for NostimintStateMachine {
    type DynType = DynState<DynGlobalClientContext>;

    fn into_dyn(self, instance_id: ModuleInstanceId) -> Self::DynType {
        DynState::from_typed(instance_id, self)
    }
}

#[derive(Error, Debug, Serialize, Deserialize, Encodable, Decodable, Clone, Eq, PartialEq)]
pub enum NostimintError {
    #[error("Nostimint module had an internal error")]
    NostimintInternalError,
}

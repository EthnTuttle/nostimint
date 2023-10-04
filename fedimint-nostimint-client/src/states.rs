use std::time::Duration;

use fedimint_client::sm::{DynState, OperationId, State, StateTransition};
use fedimint_client::transaction::TxSubmissionError;
use fedimint_client::DynGlobalClientContext;
use fedimint_core::api::GlobalFederationApi;
use fedimint_core::core::{Decoder, IntoDynInstance, ModuleInstanceId};
use fedimint_core::db::ModuleDatabaseTransaction;
use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::{Amount, OutPoint, TransactionId};
use fedimint_nostimint_common::NostimintOutputOutcome;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::NostimintClientFundsKeyV0;
use crate::{get_funds, NostimintClientContext};

/// Tracks a transaction
#[derive(Debug, Clone, Eq, PartialEq, Decodable, Encodable)]
pub enum NostimintStateMachine {
    Input(Amount, TransactionId, OperationId),
    Output(Amount, TransactionId, OperationId),
    InputDone(OperationId),
    OutputDone(Amount, OperationId),
    Refund(OperationId),
}

impl State for NostimintStateMachine {
    type ModuleContext = NostimintClientContext;
    type GlobalContext = DynGlobalClientContext;

    fn transitions(
        &self,
        context: &Self::ModuleContext,
        global_context: &Self::GlobalContext,
    ) -> Vec<StateTransition<Self>> {
        match self.clone() {
            NostimintStateMachine::Input(amount, txid, id) => vec![StateTransition::new(
                await_tx_accepted(global_context.clone(), id, txid),
                move |dbtx, res, _state: Self| match res {
                    // accepted, we are done
                    Ok(_) => Box::pin(async move { NostimintStateMachine::InputDone(id) }),
                    // tx rejected, we refund ourselves
                    Err(_) => Box::pin(async move {
                        add_funds(amount, dbtx.module_tx()).await;
                        NostimintStateMachine::Refund(id)
                    }),
                },
            )],
            NostimintStateMachine::Output(amount, txid, id) => vec![StateTransition::new(
                await_nostimint_output_outcome(
                    global_context.clone(),
                    OutPoint { txid, out_idx: 0 },
                    context.nostimint_decoder.clone(),
                ),
                move |dbtx, res, _state: Self| match res {
                    // output accepted, add funds
                    Ok(_) => Box::pin(async move {
                        add_funds(amount, dbtx.module_tx()).await;
                        NostimintStateMachine::OutputDone(amount, id)
                    }),
                    // output rejected, do not add funds
                    Err(_) => Box::pin(async move { NostimintStateMachine::Refund(id) }),
                },
            )],
            NostimintStateMachine::InputDone(_) => vec![],
            NostimintStateMachine::OutputDone(_, _) => vec![],
            NostimintStateMachine::Refund(_) => vec![],
        }
    }

    fn operation_id(&self) -> OperationId {
        match self {
            NostimintStateMachine::Input(_, _, id) => *id,
            NostimintStateMachine::Output(_, _, id) => *id,
            NostimintStateMachine::InputDone(id) => *id,
            NostimintStateMachine::OutputDone(_, id) => *id,
            NostimintStateMachine::Refund(id) => *id,
        }
    }
}

async fn add_funds(amount: Amount, mut dbtx: ModuleDatabaseTransaction<'_>) {
    let funds = get_funds(&mut dbtx).await + amount;
    dbtx.insert_entry(&NostimintClientFundsKeyV0, &funds).await;
}

// TODO: Boiler-plate, should return OutputOutcome
async fn await_tx_accepted(
    context: DynGlobalClientContext,
    id: OperationId,
    txid: TransactionId,
) -> Result<(), TxSubmissionError> {
    context.await_tx_accepted(id, txid).await
}

async fn await_nostimint_output_outcome(
    global_context: DynGlobalClientContext,
    outpoint: OutPoint,
    module_decoder: Decoder,
) -> Result<(), NostimintError> {
    global_context
        .api()
        .await_output_outcome::<NostimintOutputOutcome>(
            outpoint,
            Duration::from_millis(i32::MAX as u64),
            &module_decoder,
        )
        .await
        .map_err(|_| NostimintError::NostimintInternalError)?;
    Ok(())
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

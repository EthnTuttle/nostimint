use fedimint_core::encoding::{Decodable, Encodable};
use fedimint_core::{impl_db_record, Amount};

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum DbKeyPrefix {
    ClientFunds = 0x04,
}

#[derive(Debug, Clone, Encodable, Decodable, Eq, PartialEq, Hash)]
pub struct NostimintClientFundsKeyV0;

impl_db_record!(
    key = NostimintClientFundsKeyV0,
    value = Amount,
    db_prefix = DbKeyPrefix::ClientFunds,
);

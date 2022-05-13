//! This is the definition of a transfer messages that an application submits to a chain.

use crate::prelude::*;

use ibc_proto::google::protobuf::Any;
use ibc_proto::ibc::applications::transfer::v1::MsgTransfer as RawMsgTransfer;
use tendermint_proto::Protobuf;

use crate::applications::ics20_fungible_token_transfer::error::Error;
use crate::applications::ics20_fungible_token_transfer::IbcCoin;
use crate::core::ics02_client::height::Height;
use crate::core::ics24_host::identifier::{ChannelId, PortId};
use crate::signer::Signer;
use crate::timestamp::Timestamp;
use crate::tx_msg::Msg;

pub const TYPE_URL: &str = "/ibc.applications.transfer.v1.MsgTransfer";

/// Message definition for the "packet receiving" datagram.
#[derive(Clone, Debug, PartialEq)]
pub struct MsgTransfer {
    /// the port on which the packet will be sent
    pub source_port: PortId,
    /// the channel by which the packet will be sent
    pub source_channel: ChannelId,
    /// the tokens to be transferred
    pub token: IbcCoin,
    /// the sender address
    pub sender: Signer,
    /// the recipient address on the destination chain
    pub receiver: Signer,
    /// Timeout height relative to the current block height.
    /// The timeout is disabled when set to 0.
    pub timeout_height: Height,
    /// Timeout timestamp relative to the current block timestamp.
    /// The timeout is disabled when set to 0.
    pub timeout_timestamp: Timestamp,
}

impl Msg for MsgTransfer {
    type ValidationError = Error;
    type Raw = RawMsgTransfer;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        TYPE_URL.to_string()
    }
}

impl TryFrom<RawMsgTransfer> for MsgTransfer {
    type Error = Error;

    fn try_from(raw_msg: RawMsgTransfer) -> Result<Self, Self::Error> {
        let timeout_timestamp = Timestamp::from_nanoseconds(raw_msg.timeout_timestamp)
            .map_err(|_| Error::invalid_packet_timeout_timestamp(raw_msg.timeout_timestamp))?;

        let timeout_height = match raw_msg.timeout_height.clone() {
            None => Height::zero(),
            Some(raw_height) => raw_height.try_into().map_err(|e| {
                Error::invalid_packet_timeout_height(format!("invalid timeout height {}", e))
            })?,
        };

        let token = raw_msg.token.ok_or_else(Error::invalid_token)?.try_into()?;

        Ok(MsgTransfer {
            source_port: raw_msg
                .source_port
                .parse()
                .map_err(|e| Error::invalid_port_id(raw_msg.source_port.clone(), e))?,
            source_channel: raw_msg
                .source_channel
                .parse()
                .map_err(|e| Error::invalid_channel_id(raw_msg.source_channel.clone(), e))?,
            token,
            sender: raw_msg.sender.parse().map_err(Error::signer)?,
            receiver: raw_msg.receiver.parse().map_err(Error::signer)?,
            timeout_height,
            timeout_timestamp,
        })
    }
}

impl From<MsgTransfer> for RawMsgTransfer {
    fn from(domain_msg: MsgTransfer) -> Self {
        RawMsgTransfer {
            source_port: domain_msg.source_port.to_string(),
            source_channel: domain_msg.source_channel.to_string(),
            token: Some(domain_msg.token.into()),
            sender: domain_msg.sender.to_string(),
            receiver: domain_msg.receiver.to_string(),
            timeout_height: Some(domain_msg.timeout_height.into()),
            timeout_timestamp: domain_msg.timeout_timestamp.nanoseconds(),
        }
    }
}

impl Protobuf<RawMsgTransfer> for MsgTransfer {}

impl TryFrom<Any> for MsgTransfer {
    type Error = Error;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            TYPE_URL => MsgTransfer::decode_vec(&raw.value).map_err(Error::decode_raw_msg),
            _ => Err(Error::unknown_msg_type(raw.type_url)),
        }
    }
}

impl From<MsgTransfer> for Any {
    fn from(msg: MsgTransfer) -> Self {
        Self {
            type_url: TYPE_URL.to_string(),
            value: msg
                .encode_vec()
                .expect("encoding to `Any` from `MsgTranfer`"),
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use core::ops::Add;
    use core::time::Duration;

    use super::MsgTransfer;
    use crate::bigint::U256;
    use crate::signer::Signer;
    use crate::{
        applications::ics20_fungible_token_transfer::{BaseCoin, IbcCoin},
        core::ics24_host::identifier::{ChannelId, PortId},
        test_utils::get_dummy_bech32_account,
        timestamp::Timestamp,
        Height,
    };

    // Returns a dummy `RawMsgTransfer`, for testing only!
    pub fn get_dummy_msg_transfer(height: u64) -> MsgTransfer {
        let address: Signer = get_dummy_bech32_account().as_str().parse().unwrap();
        MsgTransfer {
            source_port: PortId::default(),
            source_channel: ChannelId::default(),
            token: IbcCoin::Base(BaseCoin {
                denom: "uatom".parse().unwrap(),
                amount: U256::from(10).into(),
            }),
            sender: address.clone(),
            receiver: address,
            timeout_timestamp: Timestamp::now().add(Duration::from_secs(10)).unwrap(),
            timeout_height: Height {
                revision_number: 0,
                revision_height: height,
            },
        }
    }
}

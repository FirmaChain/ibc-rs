use abscissa_core::clap::Parser;
use abscissa_core::{Command, Runnable};

use ibc_relayer::chain::counterparty::unreceived_packets;
use ibc_relayer::chain::handle::BaseChainHandle;
use ibc_relayer::path::PathIdentifiers;
use ibc_relayer_types::core::ics04_channel::packet::Sequence;
use ibc_relayer_types::core::ics24_host::identifier::{ChainId, ChannelId, PortId};

use crate::cli_utils::spawn_chain_counterparty;
use crate::conclude::Output;
use crate::error::Error;
use crate::prelude::*;

/// This command does the following:
/// 1. queries the chain to get its counterparty chain, channel and port identifiers (needed in 2)
/// 2. queries the counterparty chain for all packet commitments/ sequences for a given port and channel
/// 3. queries the chain for the unreceived sequences out of the list obtained in 2.
#[derive(Clone, Command, Debug, Parser, PartialEq, Eq)]
pub struct QueryPendingSendsCmd {
    #[clap(
        long = "chain",
        required = true,
        value_name = "CHAIN_ID",
        help_heading = "REQUIRED",
        help = "Identifier of the chain for the unreceived sequences"
    )]
    chain_id: ChainId,

    #[clap(
        long = "port",
        required = true,
        value_name = "PORT_ID",
        help_heading = "REQUIRED",
        help = "Port identifier"
    )]
    port_id: PortId,

    #[clap(
        long = "channel",
        visible_alias = "chan",
        required = true,
        value_name = "CHANNEL_ID",
        help_heading = "REQUIRED",
        help = "Channel identifier"
    )]
    channel_id: ChannelId,
}

impl QueryPendingSendsCmd {
    fn execute(&self) -> Result<Vec<Sequence>, Error> {
        let config = app_config();

        let (chains, chan_conn_cli) = spawn_chain_counterparty::<BaseChainHandle>(
            &config,
            &self.chain_id,
            &self.port_id,
            &self.channel_id,
        )?;

        let channel = chan_conn_cli.channel;

        debug!(
            "fetched from source chain {} the following channel {:?}",
            self.chain_id, channel
        );

        let path_identifiers = PathIdentifiers::from_channel_end(channel.clone())
            .ok_or_else(|| Error::missing_counterparty_channel_id(channel))?;

        unreceived_packets(&chains.src, &chains.dst, &path_identifiers)
            .map_err(Error::supervisor)
            .map(|(seq, _)| seq)
    }
}

impl Runnable for QueryPendingSendsCmd {
    fn run(&self) {
        match self.execute() {
            Ok(seqs) => Output::success(seqs).exit(),
            Err(e) => Output::error(format!("{}", e)).exit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QueryPendingSendsCmd;

    use std::str::FromStr;

    use abscissa_core::clap::Parser;
    use ibc_relayer_types::core::ics24_host::identifier::{ChainId, ChannelId, PortId};

    #[test]
    fn test_query_packet_unreceived_packets() {
        assert_eq!(
            QueryPendingSendsCmd {
                chain_id: ChainId::from_string("chain_id"),
                port_id: PortId::from_str("port_id").unwrap(),
                channel_id: ChannelId::from_str("channel-07").unwrap()
            },
            QueryPendingSendsCmd::parse_from([
                "test",
                "--chain",
                "chain_id",
                "--port",
                "port_id",
                "--channel",
                "channel-07"
            ])
        )
    }

    #[test]
    fn test_query_packet_unreceived_packets_chan_alias() {
        assert_eq!(
            QueryPendingSendsCmd {
                chain_id: ChainId::from_string("chain_id"),
                port_id: PortId::from_str("port_id").unwrap(),
                channel_id: ChannelId::from_str("channel-07").unwrap()
            },
            QueryPendingSendsCmd::parse_from([
                "test",
                "--chain",
                "chain_id",
                "--port",
                "port_id",
                "--chan",
                "channel-07"
            ])
        )
    }

    #[test]
    fn test_query_packet_unreceived_packets_no_chan() {
        assert!(QueryPendingSendsCmd::try_parse_from([
            "test", "--chain", "chain_id", "--port", "port_id"
        ])
        .is_err())
    }

    #[test]
    fn test_query_packet_unreceived_packets_no_port() {
        assert!(QueryPendingSendsCmd::try_parse_from([
            "test",
            "--chain",
            "chain_id",
            "--channel",
            "channel-07"
        ])
        .is_err())
    }

    #[test]
    fn test_query_packet_unreceived_packets_no_chain() {
        assert!(QueryPendingSendsCmd::try_parse_from([
            "test",
            "--port",
            "port_id",
            "--channel",
            "channel-07"
        ])
        .is_err())
    }
}

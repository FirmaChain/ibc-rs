/*!
   Functions for performing IBC transfer that works similar to
   `hermes tx raw ft-transfer`.
*/

use core::ops::Add;
use core::time::Duration;
use ibc::events::IbcEvent;
use ibc::signer::Signer;
use ibc::timestamp::Timestamp;
use ibc::Height;
use ibc_relayer::chain::cosmos::types::config::TxConfig;
use ibc_relayer::chain::handle::ChainHandle;
use ibc_relayer::transfer::{
    build_and_send_transfer_messages, build_transfer_message, Amount, TransferOptions,
};

use crate::error::{handle_generic_error, Error};
use crate::ibc::denom::Denom;
use crate::relayer::tx::simple_send_tx;
use crate::types::binary::channel::ConnectedChannel;
use crate::types::id::{TaggedChannelIdRef, TaggedPortIdRef};
use crate::types::tagged::*;
use crate::types::wallet::{Wallet, WalletAddress};

/**
   Perform the same operation as `hermes tx raw ft-transfer`.

   The function call skips the checks done in the CLI, as we already
   have the necessary information given to us by the test framework.

   Note that we cannot change the sender's wallet in this case,
   as the current `send_tx` implementation in
   [`CosmosSdkChain`](ibc_relayer::chain::cosmos::CosmosSdkChain)
   always use the signer wallet configured in the
   [`ChainConfig`](ibc_relayer::config::ChainConfig).

   Currently the only way you can transfer using a different wallet
   is to create a brand new [`ChainHandle`] with the new wallet
   specified in the [`ChainConfig`](ibc_relayer::config::ChainConfig).

   Alternatively, it is recommended that for simple case of IBC token
   transfer, test authors should instead use the
   [`transfer_token`](crate::chain::driver::transfer::transfer_token)
   function provided by [`ChainDriver`](crate::chain::driver::ChainDriver).
   That uses the `gaiad tx ibc-transfer` command to do the IBC transfer,
   which is much simpler as compared to the current way the relayer code
   is organized.
*/
pub fn tx_raw_ft_transfer<SrcChain: ChainHandle, DstChain: ChainHandle>(
    src_handle: &SrcChain,
    dst_handle: &DstChain,
    channel: &ConnectedChannel<SrcChain, DstChain>,
    recipient: &MonoTagged<DstChain, &WalletAddress>,
    denom: &MonoTagged<SrcChain, &Denom>,
    amount: u64,
    timeout_height_offset: u64,
    timeout_duration: Duration,
    number_messages: usize,
) -> Result<Vec<IbcEvent>, Error> {
    let transfer_options = TransferOptions {
        packet_src_port_id: channel.port_a.value().clone(),
        packet_src_channel_id: *channel.channel_id_a.value(),
        amount: Amount(amount.into()),
        denom: denom.value().to_string(),
        receiver: Some(recipient.value().0.clone()),
        timeout_height_offset,
        timeout_duration,
        number_msgs: number_messages,
    };

    let events = build_and_send_transfer_messages(src_handle, dst_handle, &transfer_options)?;

    Ok(events)
}

pub async fn ibc_token_transfer<SrcChain, DstChain>(
    tx_config: &MonoTagged<SrcChain, &TxConfig>,
    port_id: &TaggedPortIdRef<'_, SrcChain, DstChain>,
    channel_id: &TaggedChannelIdRef<'_, SrcChain, DstChain>,
    sender: &MonoTagged<SrcChain, &Wallet>,
    recipient: &MonoTagged<DstChain, &WalletAddress>,
    denom: &MonoTagged<SrcChain, &Denom>,
    amount: u64,
) -> Result<(), Error> {
    // During test, all chains should have the same local clock.
    // We are also not really interested in setting a timeout for most tests,
    // so we just put an approximate 30 minute timeout as the timeout
    // field is compulsary, and we want to avoid IBC timeout on CI.
    //
    // If tests require explicit timeout, they should explicitly construct the
    // transfer message and pass it to send_tx.
    let timeout_timestamp = Timestamp::now()
        .add(Duration::from_secs(30 * 60))
        .map_err(handle_generic_error)?;

    let message = build_transfer_message(
        (*port_id.value()).clone(),
        **channel_id.value(),
        amount.into(),
        denom.value().to_string(),
        Signer::new(sender.value().address.0.clone()),
        Signer::new(recipient.value().0.clone()),
        Height::zero(),
        timeout_timestamp,
    );

    simple_send_tx(tx_config.value(), &sender.value().key, vec![message]).await?;

    Ok(())
}

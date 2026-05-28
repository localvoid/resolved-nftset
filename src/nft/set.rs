use std::net::{Ipv4Addr, Ipv6Addr};

use crate::nft::NftError;
use crate::nft::netlink::{
    MsgBuffer, NFNL_MSG_BATCH_BEGIN, NFNL_MSG_BATCH_END, NFNL_SUBSYS_NFTABLES, NLM_F_ACK,
    NLM_F_CREATE, NLM_F_REQUEST, NLMSG_ERROR, NetlinkSocket, NlMsgHdr, get_nlmsg_type,
    is_nlmsg_done, parse_nlmsg_error,
};

// nftables message types
const NFT_MSG_NEWSETELEM: u16 = 12;

// nftables set element list attributes
const NFTA_SET_ELEM_LIST_TABLE: u16 = 1;
const NFTA_SET_ELEM_LIST_SET: u16 = 2;
const NFTA_SET_ELEM_LIST_ELEMENTS: u16 = 3;

// nftables set element attributes
const NFTA_SET_ELEM_KEY: u16 = 1;

// nftables data attributes
const NFTA_DATA_VALUE: u16 = 1;

// Address family constants
const NFPROTO_INET: u8 = 1;

const BUFF_SZ: usize = 2048;

/// Add an IP address to an nftables set.
pub fn nftset_add(
    table: &str,
    set_v4: &str,
    set_v6: &str,
    addrs_v4: &[Ipv4Addr],
    addrs_v6: &[Ipv6Addr],
) -> Result<(), NftError> {
    // Build the batched netlink message
    let mut buf = MsgBuffer::new(BUFF_SZ);
    // Batch begin
    buf.put_nlmsghdr(NFNL_MSG_BATCH_BEGIN, NLM_F_REQUEST, 0);
    buf.put_nfgenmsg(libc::AF_UNSPEC as u8, 0, NFNL_SUBSYS_NFTABLES as u16);
    buf.finalize_nlmsg();

    let msg_start = buf.len();
    let flags = NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE;

    if !addrs_v4.is_empty() {
        buf.put_nlmsghdr(nft_msg_type(NFT_MSG_NEWSETELEM), flags, 1);
        buf.put_nfgenmsg(NFPROTO_INET, 0, 0);
        buf.put_attr_str(NFTA_SET_ELEM_LIST_TABLE, table);
        buf.put_attr_str(NFTA_SET_ELEM_LIST_SET, set_v4);

        let elems_offset = buf.start_nested(NFTA_SET_ELEM_LIST_ELEMENTS);
        for addr in addrs_v4 {
            let elem_offset = buf.start_nested(0); // Type 0 for list item
            let key_offset = buf.start_nested(NFTA_SET_ELEM_KEY);
            buf.put_attr_bytes(NFTA_DATA_VALUE, &addr.octets());
            buf.end_nested(key_offset);
            buf.end_nested(elem_offset);
        }

        buf.end_nested(elems_offset);
        buf.finalize_nlmsg_at(msg_start);
    }
    if !addrs_v6.is_empty() {
        buf.put_nlmsghdr(nft_msg_type(NFT_MSG_NEWSETELEM), flags, 1);
        buf.put_nfgenmsg(NFPROTO_INET, 0, 0);
        buf.put_attr_str(NFTA_SET_ELEM_LIST_TABLE, table);
        buf.put_attr_str(NFTA_SET_ELEM_LIST_SET, set_v6);

        let elems_offset = buf.start_nested(NFTA_SET_ELEM_LIST_ELEMENTS);
        for addr in addrs_v6 {
            let elem_offset = buf.start_nested(0); // Type 0 for list item
            let key_offset = buf.start_nested(NFTA_SET_ELEM_KEY);
            buf.put_attr_bytes(NFTA_DATA_VALUE, &addr.octets());
            buf.end_nested(key_offset);
            buf.end_nested(elem_offset);
        }

        buf.end_nested(elems_offset);
        buf.finalize_nlmsg_at(msg_start);
    }

    // Batch end message
    let end_start = buf.len();
    buf.put_nlmsghdr(NFNL_MSG_BATCH_END, NLM_F_REQUEST, 2);
    buf.put_nfgenmsg(libc::AF_UNSPEC as u8, 0, NFNL_SUBSYS_NFTABLES as u16);
    buf.finalize_nlmsg_at(end_start);

    // Send and receive
    let socket = NetlinkSocket::new()?;
    socket.send(buf.as_slice())?;

    // Receive all responses
    let mut recv_buf = [0u8; BUFF_SZ];
    loop {
        let recv_len = socket.recv(&mut recv_buf)?;

        if recv_len < NlMsgHdr::SIZE {
            return Err(NftError::ProtocolError);
        }

        // Check for error
        if let Some(error) = parse_nlmsg_error(&recv_buf[..recv_len]) {
            if error == 0 {
                // Continue reading
            } else {
                match -error {
                    libc::ENOENT => {
                        return Err(NftError::SetNotFound);
                    }
                    _ => return Err(NftError::NetlinkError(-error)),
                }
            }
        }

        // Check for NLMSG_DONE
        if is_nlmsg_done(&recv_buf[..recv_len]) {
            break;
        }

        // Check message type to determine if we should continue
        let msg_type = get_nlmsg_type(&recv_buf[..recv_len]);
        if msg_type == Some(NLMSG_ERROR) {
            break;
        }
    }

    Ok(())
}

/// Build the netlink message type for nftables commands.
#[inline]
const fn nft_msg_type(cmd: u16) -> u16 {
    ((NFNL_SUBSYS_NFTABLES as u16) << 8) | cmd
}

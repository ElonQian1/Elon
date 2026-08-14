use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use zeroize::{Zeroize, Zeroizing};

use super::crypto::{mac_tag, verify_mac, Secret32};

const FRAME_MAGIC: &[u8; 4] = b"ELSP";
const FRAME_VERSION: u8 = 1;
const FRAME_FLAGS: u16 = 0;
const FRAME_HEADER_BYTES: usize = 20;
const FRAME_TAG_BYTES: usize = 32;
const MAX_CONTROL_PAYLOAD_BYTES: usize = 1_048_576;
const MAX_CONFIG_PAYLOAD_BYTES: usize = 1_048_576;
const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = 65_536;
const MAX_PAYLOAD_BYTES: usize = MAX_CONTROL_PAYLOAD_BYTES;
const MAX_PACKET_BYTES: usize = FRAME_HEADER_BYTES + MAX_PAYLOAD_BYTES + FRAME_TAG_BYTES;
const MAX_FRAMES_PER_DIRECTION: u64 = 1_048_576;
const FRAME_IO_TIMEOUT: Duration = Duration::from_millis(5_000);
const FRAME_MAC_LABEL: &[u8] = b"frame\0";

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ExternalPoolAdapterSessionFrameKind {
    Control = 1,
    Config = 2,
    Credential = 3,
}

impl ExternalPoolAdapterSessionFrameKind {
    fn from_byte(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Control),
            2 => Ok(Self::Config),
            3 => Ok(Self::Credential),
            _ => bail!("authenticated session frame rejected"),
        }
    }

    pub const fn maximum_payload_bytes(self) -> usize {
        match self {
            Self::Control => MAX_CONTROL_PAYLOAD_BYTES,
            Self::Config => MAX_CONFIG_PAYLOAD_BYTES,
            Self::Credential => MAX_CREDENTIAL_PAYLOAD_BYTES,
        }
    }
}

pub struct AuthenticatedExternalPoolAdapterSessionFrame {
    kind: ExternalPoolAdapterSessionFrameKind,
    payload: Zeroizing<Vec<u8>>,
}

impl AuthenticatedExternalPoolAdapterSessionFrame {
    pub fn kind(&self) -> ExternalPoolAdapterSessionFrameKind {
        self.kind
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

pub struct AuthenticatedExternalPoolAdapterSession {
    socket: OwnedFd,
    send_key: Secret32,
    receive_key: Secret32,
    send_direction: u8,
    receive_direction: u8,
    transcript_digest: [u8; 32],
    next_send_sequence: u64,
    next_receive_sequence: u64,
    active: bool,
    terminal_strategy: SocketTerminalStrategy,
}

#[derive(Clone, Copy)]
pub(crate) enum SocketTerminalStrategy {
    ShutdownAndClose,
    CloseOnly,
}

impl SocketTerminalStrategy {
    pub(crate) fn terminate(self, fd: RawFd) {
        if matches!(self, Self::ShutdownAndClose) {
            unsafe {
                libc::shutdown(fd, libc::SHUT_RDWR);
            }
        }
    }
}

impl AuthenticatedExternalPoolAdapterSession {
    pub(super) fn new(
        socket: OwnedFd,
        send_key: Secret32,
        receive_key: Secret32,
        send_direction: u8,
        receive_direction: u8,
        transcript_digest: [u8; 32],
        terminal_strategy: SocketTerminalStrategy,
    ) -> Self {
        Self {
            socket,
            send_key,
            receive_key,
            send_direction,
            receive_direction,
            transcript_digest,
            next_send_sequence: 1,
            next_receive_sequence: 1,
            active: true,
            terminal_strategy,
        }
    }

    pub fn send(
        &mut self,
        kind: ExternalPoolAdapterSessionFrameKind,
        payload: &[u8],
    ) -> Result<()> {
        let result = self.send_inner(kind, payload);
        if let Err(error) = result {
            return self.fail(error);
        }
        Ok(())
    }

    pub fn receive(&mut self) -> Result<AuthenticatedExternalPoolAdapterSessionFrame> {
        let result = self.receive_inner();
        match result {
            Ok(frame) => Ok(frame),
            Err(error) => self.fail(error),
        }
    }

    fn send_inner(
        &mut self,
        kind: ExternalPoolAdapterSessionFrameKind,
        payload: &[u8],
    ) -> Result<()> {
        self.ensure_active()?;
        if payload.len() > kind.maximum_payload_bytes() {
            bail!("authenticated session frame rejected");
        }
        if self.next_send_sequence > MAX_FRAMES_PER_DIRECTION {
            bail!("authenticated session sequence exhausted");
        }
        let packet = encode_frame(
            &self.send_key,
            self.send_direction,
            &self.transcript_digest,
            kind as u8,
            self.next_send_sequence,
            payload,
        )?;
        send_packet(self.socket.as_raw_fd(), &packet, FRAME_IO_TIMEOUT)?;
        self.next_send_sequence = self
            .next_send_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("authenticated session sequence overflow"))?;
        Ok(())
    }

    fn receive_inner(&mut self) -> Result<AuthenticatedExternalPoolAdapterSessionFrame> {
        self.ensure_active()?;
        let packet = receive_packet(self.socket.as_raw_fd(), MAX_PACKET_BYTES, FRAME_IO_TIMEOUT)?;
        if packet.len() < FRAME_HEADER_BYTES + FRAME_TAG_BYTES {
            bail!("authenticated session frame rejected");
        }
        let header = &packet[..FRAME_HEADER_BYTES];
        if &header[..4] != FRAME_MAGIC || header[4] != FRAME_VERSION {
            bail!("authenticated session frame rejected");
        }
        let flags = u16::from_be_bytes(header[6..8].try_into().expect("fixed flags range"));
        if flags != FRAME_FLAGS {
            bail!("authenticated session frame rejected");
        }
        let sequence = u64::from_be_bytes(header[8..16].try_into().expect("fixed sequence range"));
        let payload_length =
            u32::from_be_bytes(header[16..20].try_into().expect("fixed payload range")) as usize;
        if payload_length > MAX_PAYLOAD_BYTES
            || packet.len() != FRAME_HEADER_BYTES + payload_length + FRAME_TAG_BYTES
        {
            bail!("authenticated session frame rejected");
        }
        let payload_end = FRAME_HEADER_BYTES + payload_length;
        let payload = &packet[FRAME_HEADER_BYTES..payload_end];
        verify_mac(
            &self.receive_key,
            self.receive_direction,
            &self.transcript_digest,
            FRAME_MAC_LABEL,
            &[header, payload],
            &packet[payload_end..],
        )?;
        let kind = ExternalPoolAdapterSessionFrameKind::from_byte(header[5])?;
        if payload_length > kind.maximum_payload_bytes()
            || sequence != self.next_receive_sequence
            || sequence > MAX_FRAMES_PER_DIRECTION
        {
            bail!("authenticated session frame rejected");
        }
        self.next_receive_sequence = self
            .next_receive_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("authenticated session sequence overflow"))?;
        Ok(AuthenticatedExternalPoolAdapterSessionFrame {
            kind,
            payload: Zeroizing::new(payload.to_vec()),
        })
    }

    fn ensure_active(&self) -> Result<()> {
        if !self.active {
            bail!("authenticated session is terminal");
        }
        Ok(())
    }

    /// Makes a locally detected protocol or authority mismatch terminal before returning it.
    pub fn terminate(&mut self) {
        if self.active {
            self.active = false;
            self.send_key.zeroize_now();
            self.receive_key.zeroize_now();
            self.terminal_strategy.terminate(self.socket.as_raw_fd());
        }
    }

    fn fail<T>(&mut self, error: anyhow::Error) -> Result<T> {
        self.terminate();
        Err(error)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn encode_outgoing_for_test(&self, kind: u8, sequence: u64, payload: &[u8]) -> Vec<u8> {
        encode_frame(
            &self.send_key,
            self.send_direction,
            &self.transcript_digest,
            kind,
            sequence,
            payload,
        )
        .expect("encode outgoing test packet")
        .to_vec()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn encode_incoming_for_test(&self, kind: u8, sequence: u64, payload: &[u8]) -> Vec<u8> {
        encode_frame(
            &self.receive_key,
            self.receive_direction,
            &self.transcript_digest,
            kind,
            sequence,
            payload,
        )
        .expect("encode incoming test packet")
        .to_vec()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn send_raw_for_test(&self, packet: &[u8]) -> Result<()> {
        send_packet(self.socket.as_raw_fd(), packet, FRAME_IO_TIMEOUT)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn socket_fd_for_test(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl Drop for AuthenticatedExternalPoolAdapterSession {
    fn drop(&mut self) {
        self.send_key.zeroize_now();
        self.receive_key.zeroize_now();
        self.terminal_strategy.terminate(self.socket.as_raw_fd());
    }
}

fn encode_frame(
    key: &Secret32,
    direction: u8,
    transcript_digest: &[u8; 32],
    kind: u8,
    sequence: u64,
    payload: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let payload_length = u32::try_from(payload.len()).context("session payload length")?;
    let mut packet = Zeroizing::new(Vec::with_capacity(
        FRAME_HEADER_BYTES + payload.len() + FRAME_TAG_BYTES,
    ));
    packet.extend_from_slice(FRAME_MAGIC);
    packet.push(FRAME_VERSION);
    packet.push(kind);
    packet.extend_from_slice(&FRAME_FLAGS.to_be_bytes());
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(&payload_length.to_be_bytes());
    packet.extend_from_slice(payload);
    let tag = mac_tag(
        key,
        direction,
        transcript_digest,
        FRAME_MAC_LABEL,
        &[&packet[..FRAME_HEADER_BYTES], payload],
    )?;
    packet.extend_from_slice(&tag);
    Ok(packet)
}

pub(super) fn create_seqpacket_pair() -> Result<(OwnedFd, OwnedFd)> {
    let mut descriptors = [-1; 2];
    let socket_type = libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
    if unsafe { libc::socketpair(libc::AF_UNIX, socket_type, 0, descriptors.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error())
            .context("create anonymous SOCK_SEQPACKET pair");
    }
    let first = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let second = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    Ok((first, second))
}

pub(super) fn send_packet(fd: RawFd, packet: &[u8], timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        let mut payload = libc::iovec {
            iov_base: packet.as_ptr().cast_mut().cast(),
            iov_len: packet.len(),
        };
        let mut message = unsafe { std::mem::zeroed::<libc::msghdr>() };
        message.msg_iov = &mut payload;
        message.msg_iovlen = 1;
        let written = unsafe { libc::sendmsg(fd, &message, libc::MSG_NOSIGNAL) };
        if written == packet.len() as isize {
            return Ok(());
        }
        if written >= 0 {
            bail!("SOCK_SEQPACKET send was not atomic");
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EAGAIN) => wait_ready(fd, libc::POLLOUT, deadline)?,
            _ => return Err(error).context("send authenticated seqpacket"),
        }
    }
}

pub(super) fn receive_packet(
    fd: RawFd,
    maximum_bytes: usize,
    timeout: Duration,
) -> Result<Zeroizing<Vec<u8>>> {
    let deadline = Instant::now() + timeout;
    let mut packet = Zeroizing::new(vec![0_u8; maximum_bytes]);
    loop {
        let mut payload = libc::iovec {
            iov_base: packet.as_mut_ptr().cast(),
            iov_len: maximum_bytes,
        };
        let mut message = unsafe { std::mem::zeroed::<libc::msghdr>() };
        message.msg_iov = &mut payload;
        message.msg_iovlen = 1;
        let received = unsafe { libc::recvmsg(fd, &mut message, libc::MSG_TRUNC) };
        if received >= 0 {
            let received = received as usize;
            if received > maximum_bytes {
                bail!("authenticated seqpacket exceeds the server-fixed limit");
            }
            packet.truncate(received);
            return Ok(packet);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EAGAIN) => wait_ready(fd, libc::POLLIN, deadline)?,
            _ => return Err(error).context("receive authenticated seqpacket"),
        }
    }
}

fn wait_ready(fd: RawFd, events: libc::c_short, deadline: Instant) -> Result<()> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            bail!("authenticated seqpacket timed out");
        }
        let timeout_ms = i32::try_from(remaining.as_millis().min(i32::MAX as u128))
            .unwrap_or(i32::MAX)
            .max(1);
        let mut descriptor = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
        if ready > 0 {
            if descriptor.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                bail!("authenticated seqpacket became invalid");
            }
            if descriptor.revents & (events | libc::POLLHUP) != 0 {
                return Ok(());
            }
        } else if ready == 0 {
            bail!("authenticated seqpacket timed out");
        } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return Err(std::io::Error::last_os_error()).context("poll authenticated seqpacket");
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub const TEST_MAX_PACKET_BYTES: usize = MAX_PACKET_BYTES;

use std::{
    os::fd::{FromRawFd, OwnedFd, RawFd},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use zeroize::Zeroizing;

pub(crate) fn create_seqpacket_pair() -> Result<(OwnedFd, OwnedFd)> {
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

pub(crate) fn send_packet(fd: RawFd, packet: &[u8], timeout: Duration) -> Result<()> {
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

pub(crate) fn receive_packet(
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
            if message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0 {
                bail!("authenticated seqpacket was truncated");
            }
            if message.msg_controllen != 0 {
                bail!("authenticated seqpacket carried unexpected control data");
            }
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

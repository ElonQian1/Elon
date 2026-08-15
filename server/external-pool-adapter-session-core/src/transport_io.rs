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

#[cfg(test)]
mod tests {
    use std::{mem::size_of, os::fd::AsRawFd, time::Duration};

    use super::*;

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);

    #[test]
    fn receive_accepts_plain_seqpacket_within_fixed_limit() {
        let (sender, receiver) = create_seqpacket_pair().expect("create seqpacket pair");
        send_packet(sender.as_raw_fd(), b"plain", TEST_TIMEOUT).expect("send plain packet");

        let packet =
            receive_packet(receiver.as_raw_fd(), 5, TEST_TIMEOUT).expect("receive plain packet");

        assert_eq!(&*packet, b"plain");
    }

    #[test]
    fn receive_rejects_seqpacket_larger_than_server_fixed_limit() {
        let (sender, receiver) = create_seqpacket_pair().expect("create seqpacket pair");
        send_packet(sender.as_raw_fd(), b"oversize", TEST_TIMEOUT).expect("send oversized packet");

        let error = receive_packet(receiver.as_raw_fd(), 4, TEST_TIMEOUT)
            .expect_err("oversized packet must fail closed");

        assert!(error.to_string().contains("truncated"));
    }

    #[test]
    fn receive_rejects_scm_rights_control_data() {
        let (sender, receiver) = create_seqpacket_pair().expect("create seqpacket pair");
        let passed_fd = unsafe { libc::dup(sender.as_raw_fd()) };
        assert!(passed_fd >= 0, "duplicate descriptor for SCM_RIGHTS");
        let passed_fd = unsafe { OwnedFd::from_raw_fd(passed_fd) };

        send_fd_control_packet(sender.as_raw_fd(), passed_fd.as_raw_fd());
        let error = receive_packet(receiver.as_raw_fd(), 1, TEST_TIMEOUT)
            .expect_err("SCM_RIGHTS packet must fail closed");

        assert!(error.to_string().contains("truncated"));
    }

    #[test]
    fn receive_rejects_kernel_generated_credentials() {
        let (sender, receiver) = create_seqpacket_pair().expect("create seqpacket pair");
        let enabled: libc::c_int = 1;
        let result = unsafe {
            libc::setsockopt(
                receiver.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PASSCRED,
                (&enabled as *const libc::c_int).cast(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        assert_eq!(result, 0, "enable SO_PASSCRED on receiver");
        send_packet(sender.as_raw_fd(), b"c", TEST_TIMEOUT)
            .expect("send credential-bearing packet");

        let error = receive_packet(receiver.as_raw_fd(), 1, TEST_TIMEOUT)
            .expect_err("SCM_CREDENTIALS packet must fail closed");

        assert!(error.to_string().contains("truncated"));
    }

    fn send_fd_control_packet(socket: RawFd, passed_fd: RawFd) {
        let mut payload_byte = [b'f'];
        let mut payload = libc::iovec {
            iov_base: payload_byte.as_mut_ptr().cast(),
            iov_len: payload_byte.len(),
        };
        let control_bytes = unsafe { libc::CMSG_SPACE(size_of::<RawFd>() as u32) } as usize;
        let word_bytes = size_of::<usize>();
        let mut control = vec![0_usize; control_bytes.div_ceil(word_bytes)];
        let mut message = unsafe { std::mem::zeroed::<libc::msghdr>() };
        message.msg_iov = &mut payload;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = control_bytes;

        let header = unsafe { libc::CMSG_FIRSTHDR(&message) };
        assert!(!header.is_null(), "SCM_RIGHTS header must fit");
        unsafe {
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(size_of::<RawFd>() as u32) as usize;
            std::ptr::write_unaligned(libc::CMSG_DATA(header).cast::<RawFd>(), passed_fd);
        }

        let written = unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) };
        assert_eq!(written, 1, "send SCM_RIGHTS packet");
    }
}

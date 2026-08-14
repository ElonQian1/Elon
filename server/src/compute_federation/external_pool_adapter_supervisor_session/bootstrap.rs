use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use ring::constant_time;
use zeroize::Zeroizing;

use super::{
    crypto::{
        derive_directional_keys, mac_tag, random_array32, random_secret32, verify_mac, Secret32,
        CHILD_TO_HOST_DIRECTION, HOST_TO_CHILD_DIRECTION,
    },
    roots::ExternalPoolAdapterSessionRoots,
    transport::{
        create_seqpacket_pair, receive_packet, send_packet, AuthenticatedExternalPoolAdapterSession,
    },
};

const BOOTSTRAP_TIMEOUT: Duration = Duration::from_millis(5_000);
const CHALLENGE_MAGIC: &[u8; 4] = b"ELS0";
const RESPONSE_MAGIC: &[u8; 4] = b"ELS1";
const CONFIRM_MAGIC: &[u8; 4] = b"ELS2";
const BOOTSTRAP_VERSION: u8 = 1;
const CHALLENGE_BYTES: usize = 4 + 1 + 32 + 32;
const RESPONSE_PREFIX_BYTES: usize = 4 + 1 + 32;
const RESPONSE_BYTES: usize = RESPONSE_PREFIX_BYTES + 32;
const CONFIRM_PREFIX_BYTES: usize = 4 + 1;
const CONFIRM_BYTES: usize = CONFIRM_PREFIX_BYTES + 32;
const RESPONSE_PROOF_LABEL: &[u8] = b"bootstrap_response\0";
const CONFIRM_PROOF_LABEL: &[u8] = b"bootstrap_confirm\0";

pub(in crate::compute_federation) struct PreparedExternalPoolAdapterSupervisorSession {
    host: ExternalPoolAdapterHostBootstrap,
    child: ExternalPoolAdapterChildBootstrap,
}

impl PreparedExternalPoolAdapterSupervisorSession {
    pub(in crate::compute_federation) fn split(
        self,
    ) -> (
        ExternalPoolAdapterHostBootstrap,
        ExternalPoolAdapterChildBootstrap,
    ) {
        (self.host, self.child)
    }
}

pub(in crate::compute_federation) struct ExternalPoolAdapterHostBootstrap {
    socket: OwnedFd,
    seed: Secret32,
    host_nonce: [u8; 32],
    roots: ExternalPoolAdapterSessionRoots,
}

pub(in crate::compute_federation) struct ExternalPoolAdapterChildBootstrap {
    socket: OwnedFd,
    seed_reader: OwnedFd,
}

pub(in crate::compute_federation) fn prepare_external_pool_adapter_supervisor_session(
    roots: ExternalPoolAdapterSessionRoots,
) -> Result<PreparedExternalPoolAdapterSupervisorSession> {
    let (host_socket, child_socket) = create_seqpacket_pair()?;
    let seed = random_secret32()?;
    let child_seed_reader = create_seed_channel(seed.as_bytes())?;
    Ok(PreparedExternalPoolAdapterSupervisorSession {
        host: ExternalPoolAdapterHostBootstrap {
            socket: host_socket,
            seed,
            host_nonce: random_array32()?,
            roots,
        },
        child: ExternalPoolAdapterChildBootstrap {
            socket: child_socket,
            seed_reader: child_seed_reader,
        },
    })
}

impl ExternalPoolAdapterHostBootstrap {
    pub(in crate::compute_federation) fn authenticate(
        self,
    ) -> Result<AuthenticatedExternalPoolAdapterSession> {
        let transcript_digest = self.roots.transcript_digest();
        let challenge = encode_challenge(&self.host_nonce, &transcript_digest);
        send_packet(self.socket.as_raw_fd(), &challenge, BOOTSTRAP_TIMEOUT)?;
        let response = receive_exact(self.socket.as_raw_fd(), RESPONSE_BYTES)?;
        if &response[..4] != RESPONSE_MAGIC || response[4] != BOOTSTRAP_VERSION {
            return fail_bootstrap(&self.socket);
        }
        let child_nonce: [u8; 32] = response[5..RESPONSE_PREFIX_BYTES]
            .try_into()
            .expect("fixed child nonce range");
        let keys =
            derive_directional_keys(&self.seed, &self.host_nonce, &child_nonce, &self.roots)?;
        let (send_key, receive_key, transcript_digest) = keys.into_host();
        if verify_mac(
            &receive_key,
            CHILD_TO_HOST_DIRECTION,
            &transcript_digest,
            RESPONSE_PROOF_LABEL,
            &[&challenge, &response[..RESPONSE_PREFIX_BYTES]],
            &response[RESPONSE_PREFIX_BYTES..],
        )
        .is_err()
        {
            return fail_bootstrap(&self.socket);
        }
        let mut confirmation = Zeroizing::new(Vec::with_capacity(CONFIRM_BYTES));
        confirmation.extend_from_slice(CONFIRM_MAGIC);
        confirmation.push(BOOTSTRAP_VERSION);
        let proof = mac_tag(
            &send_key,
            HOST_TO_CHILD_DIRECTION,
            &transcript_digest,
            CONFIRM_PROOF_LABEL,
            &[&challenge, &response],
        )?;
        confirmation.extend_from_slice(&proof);
        send_packet(self.socket.as_raw_fd(), &confirmation, BOOTSTRAP_TIMEOUT)?;
        Ok(AuthenticatedExternalPoolAdapterSession::new(
            self.socket,
            send_key,
            receive_key,
            HOST_TO_CHILD_DIRECTION,
            CHILD_TO_HOST_DIRECTION,
            transcript_digest,
        ))
    }
}

impl ExternalPoolAdapterChildBootstrap {
    pub(in crate::compute_federation) fn authenticate(
        self,
        roots: ExternalPoolAdapterSessionRoots,
    ) -> Result<AuthenticatedExternalPoolAdapterSession> {
        self.authenticate_inner(roots, false, false)
    }

    fn authenticate_inner(
        self,
        roots: ExternalPoolAdapterSessionRoots,
        corrupt_seed: bool,
        tamper_response_proof: bool,
    ) -> Result<AuthenticatedExternalPoolAdapterSession> {
        let mut seed = read_seed(self.seed_reader)?;
        if corrupt_seed {
            let mut corrupt = *seed.as_bytes();
            corrupt[0] ^= 0x80;
            seed.zeroize_now();
            seed = Secret32::new(corrupt);
        }
        let challenge = receive_exact(self.socket.as_raw_fd(), CHALLENGE_BYTES)?;
        if &challenge[..4] != CHALLENGE_MAGIC || challenge[4] != BOOTSTRAP_VERSION {
            return fail_bootstrap(&self.socket);
        }
        let host_nonce: [u8; 32] = challenge[5..37].try_into().expect("fixed host nonce range");
        let expected_transcript = roots.transcript_digest();
        if constant_time::verify_slices_are_equal(&challenge[37..], &expected_transcript).is_err() {
            return fail_bootstrap(&self.socket);
        }
        let child_nonce = random_array32()?;
        let keys = derive_directional_keys(&seed, &host_nonce, &child_nonce, &roots)?;
        seed.zeroize_now();
        let (send_key, receive_key, transcript_digest) = keys.into_child();
        let mut response = Zeroizing::new(Vec::with_capacity(RESPONSE_BYTES));
        response.extend_from_slice(RESPONSE_MAGIC);
        response.push(BOOTSTRAP_VERSION);
        response.extend_from_slice(&child_nonce);
        let mut proof = mac_tag(
            &send_key,
            CHILD_TO_HOST_DIRECTION,
            &transcript_digest,
            RESPONSE_PROOF_LABEL,
            &[&challenge, &response],
        )?;
        if tamper_response_proof {
            proof[0] ^= 0x80;
        }
        response.extend_from_slice(&proof);
        send_packet(self.socket.as_raw_fd(), &response, BOOTSTRAP_TIMEOUT)?;
        let confirmation = receive_exact(self.socket.as_raw_fd(), CONFIRM_BYTES)?;
        if &confirmation[..4] != CONFIRM_MAGIC || confirmation[4] != BOOTSTRAP_VERSION {
            return fail_bootstrap(&self.socket);
        }
        if verify_mac(
            &receive_key,
            HOST_TO_CHILD_DIRECTION,
            &transcript_digest,
            CONFIRM_PROOF_LABEL,
            &[&challenge, &response],
            &confirmation[CONFIRM_PREFIX_BYTES..],
        )
        .is_err()
        {
            return fail_bootstrap(&self.socket);
        }
        Ok(AuthenticatedExternalPoolAdapterSession::new(
            self.socket,
            send_key,
            receive_key,
            CHILD_TO_HOST_DIRECTION,
            HOST_TO_CHILD_DIRECTION,
            transcript_digest,
        ))
    }

    #[cfg(test)]
    pub(super) fn authenticate_with_wrong_seed_for_test(
        self,
        roots: ExternalPoolAdapterSessionRoots,
    ) -> Result<AuthenticatedExternalPoolAdapterSession> {
        self.authenticate_inner(roots, true, false)
    }

    #[cfg(test)]
    pub(super) fn authenticate_with_tampered_proof_for_test(
        self,
        roots: ExternalPoolAdapterSessionRoots,
    ) -> Result<AuthenticatedExternalPoolAdapterSession> {
        self.authenticate_inner(roots, false, true)
    }

    #[cfg(test)]
    pub(super) fn socket_fd_for_test(&self) -> RawFd {
        self.socket.as_raw_fd()
    }

    #[cfg(test)]
    pub(super) fn seed_fd_for_test(&self) -> RawFd {
        self.seed_reader.as_raw_fd()
    }
}

fn encode_challenge(host_nonce: &[u8; 32], transcript_digest: &[u8; 32]) -> [u8; 69] {
    let mut challenge = [0_u8; CHALLENGE_BYTES];
    challenge[..4].copy_from_slice(CHALLENGE_MAGIC);
    challenge[4] = BOOTSTRAP_VERSION;
    challenge[5..37].copy_from_slice(host_nonce);
    challenge[37..].copy_from_slice(transcript_digest);
    challenge
}

fn receive_exact(fd: RawFd, exact_bytes: usize) -> Result<Zeroizing<Vec<u8>>> {
    let packet = receive_packet(fd, exact_bytes + 1, BOOTSTRAP_TIMEOUT)?;
    if packet.len() != exact_bytes {
        bail!("authenticated session bootstrap rejected");
    }
    Ok(packet)
}

fn create_seed_channel(seed: &[u8; 32]) -> Result<OwnedFd> {
    let mut descriptors = [-1; 2];
    if unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error()).context("create anonymous seed channel");
    }
    let reader = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    write_all(writer.as_raw_fd(), seed)?;
    drop(writer);
    Ok(reader)
}

fn write_all(fd: RawFd, bytes: &[u8]) -> Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let written =
            unsafe { libc::write(fd, bytes[offset..].as_ptr().cast(), bytes.len() - offset) };
        if written > 0 {
            offset += written as usize;
        } else if written == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
        {
            continue;
        } else {
            return Err(std::io::Error::last_os_error()).context("write one-time session seed");
        }
    }
    Ok(())
}

fn read_seed(reader: OwnedFd) -> Result<Secret32> {
    let mut seed = [0_u8; 32];
    let mut offset = 0;
    while offset < seed.len() {
        let read = unsafe {
            libc::read(
                reader.as_raw_fd(),
                seed[offset..].as_mut_ptr().cast(),
                seed.len() - offset,
            )
        };
        if read > 0 {
            offset += read as usize;
        } else if read == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
        {
            continue;
        } else {
            seed.fill(0);
            bail!("one-time session seed was not exactly 32 bytes");
        }
    }
    let mut trailing = [0_u8; 1];
    let trailing_read = unsafe {
        libc::read(
            reader.as_raw_fd(),
            trailing.as_mut_ptr().cast(),
            trailing.len(),
        )
    };
    drop(reader);
    if trailing_read != 0 {
        seed.fill(0);
        bail!("one-time session seed contained trailing bytes");
    }
    Ok(Secret32::new(seed))
}

fn fail_bootstrap<T>(socket: &OwnedFd) -> Result<T> {
    unsafe {
        libc::shutdown(socket.as_raw_fd(), libc::SHUT_RDWR);
    }
    Err(anyhow!("authenticated session bootstrap rejected"))
}

use crate::core::ExecOutcome;
use crate::low_level::reactor::Fd;
use crate::low_level::reactor::{Event, Token};
use crate::low_level::spawn::SysError;
use libc::{
    SO_PEERCRED, SOCK_CLOEXEC, SOCK_NONBLOCK, SOL_SOCKET, accept4, c_void, socklen_t, ucred,
};
use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, RawFd};

const MAX_CLIENTS: usize = 32;
const MAX_PACKET_SIZE: usize = 128 * 1024; // 128 KB
const MAX_READ_BUF: usize = 256 * 1024; // 256 KB
const MAX_WRITE_BUF: usize = 1024 * 1024; // 1 MB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadState {
    Header { needed: usize },
    Body { len: usize },
}

pub struct Conn {
    pub fd: Fd,
    pub token: Token,
    pub read_buf: Vec<u8>,
    pub write_buf: Vec<u8>,
    pub state: ReadState,
    pub uid: u32,
}

pub struct WireMsg {
    pub client_id: u32,
    pub command: Command,
    pub uid: u32,
}

pub struct IpcModule {
    pub fd: Fd,
    pub server_token: Option<Token>,

    pub clients: HashMap<u32, Conn>,
    pub client_tokens: HashMap<Token, u32>,
    next_client_id: u32,
}

impl IpcModule {
    pub fn new(fd: Fd, token: Token) -> Self {
        Self {
            fd,
            server_token: Some(token),

            clients: HashMap::new(),
            client_tokens: HashMap::new(),
            next_client_id: 1,
        }
    }

    pub fn verify_peer_credentials(&self, peer_fd: RawFd) -> Result<u32, SysError> {
        let mut cred: ucred = unsafe { std::mem::zeroed() };
        let mut len: socklen_t = std::mem::size_of::<ucred>() as socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                peer_fd,
                SOL_SOCKET,
                SO_PEERCRED,
                &mut cred as *mut ucred as *mut c_void,
                &mut len as *mut socklen_t,
            )
        };

        if ret != 0 {
            return Err(SysError::sys(
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
                "getsockopt(SO_PEERCRED)",
            ));
        }

        Ok(cred.uid)
    }

    pub fn accept_clients(&mut self, reactor: &mut crate::low_level::reactor::Reactor) {
        loop {
            if self.clients.len() >= MAX_CLIENTS {
                return;
            }

            let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            let mut addr_len: socklen_t = std::mem::size_of::<libc::sockaddr_un>() as socklen_t;

            let client_fd = unsafe {
                accept4(
                    self.fd.as_raw_fd(),
                    &mut addr as *mut libc::sockaddr_un as *mut libc::sockaddr,
                    &mut addr_len as *mut socklen_t,
                    SOCK_NONBLOCK | SOCK_CLOEXEC,
                )
            };

            if client_fd < 0 {
                let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                    return;
                }
                return;
            }

            if let Ok(client_fd_obj) = Fd::new(client_fd, "accept4") {
                let uid = match self.verify_peer_credentials(client_fd) {
                    Ok(u) => u,
                    Err(_) => continue,
                };

                let token = match reactor.add(&client_fd_obj, true, true) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let client_id = self.next_client_id;
                self.next_client_id = self.next_client_id.wrapping_add(1);
                if self.next_client_id == 0 {
                    self.next_client_id = 1;
                }

                let conn = Conn {
                    fd: client_fd_obj,
                    token,
                    read_buf: Vec::with_capacity(4096),
                    write_buf: Vec::with_capacity(4096),
                    state: ReadState::Header { needed: 4 },
                    uid,
                };

                self.clients.insert(client_id, conn);
                self.client_tokens.insert(token, client_id);
            }
        }
    }

    pub fn handle_event(
        &mut self,
        reactor: &mut crate::low_level::reactor::Reactor,
        event: &Event,
    ) -> Vec<WireMsg> {
        if Some(event.token) == self.server_token && event.readable {
            self.accept_clients(reactor);
            return Vec::new();
        }

        let client_id = match self.client_tokens.get(&event.token) {
            Some(&id) => id,
            None => return Vec::new(),
        };

        if event.error {
            self.disconnect(client_id, reactor);
            return Vec::new();
        }

        let mut should_disconnect = false;

        if event.readable
            && let Some(conn) = self.clients.get_mut(&client_id)
        {
            let mut buf = [0u8; 4096];
            loop {
                match conn.fd.read(buf.as_mut_ptr(), buf.len()) {
                    Ok(0) => {
                        should_disconnect = true;
                        break;
                    }
                    Ok(n) => {
                        conn.read_buf.extend_from_slice(&buf[..n]);
                        if conn.read_buf.len() > MAX_READ_BUF {
                            should_disconnect = true;
                            break;
                        }
                    }
                    Err(e) => {
                        let raw_err = e.raw_os_error();
                        if raw_err == Some(libc::EAGAIN) || raw_err == Some(libc::EWOULDBLOCK) {
                            break;
                        } else {
                            should_disconnect = true;
                            break;
                        }
                    }
                }
            }

            if !should_disconnect {
                loop {
                    match conn.state {
                        ReadState::Header { needed } => {
                            if conn.read_buf.len() >= needed {
                                let mut len_buf = [0u8; 4];
                                len_buf.copy_from_slice(&conn.read_buf[..4]);
                                let body_len = u32::from_le_bytes(len_buf) as usize;

                                if body_len > MAX_PACKET_SIZE || body_len == 0 {
                                    should_disconnect = true;
                                    break;
                                }

                                conn.read_buf.drain(..4);
                                conn.state = ReadState::Body { len: body_len };
                            } else {
                                break;
                            }
                        }
                        ReadState::Body { len } => {
                            if conn.read_buf.len() >= len {
                                let payload = conn.read_buf.drain(..len).collect::<Vec<_>>();
                                conn.state = ReadState::Header { needed: 4 };

                                if !payload.is_empty() {
                                    let req_type = payload[0];
                                    let req = match req_type {
                                        1 => serde_json::from_slice::<Command>(&payload[1..]).ok(),
                                        2 => {
                                            if payload.len() == 9 {
                                                let mut id_buf = [0u8; 8];
                                                id_buf.copy_from_slice(&payload[1..9]);
                                                let id = u64::from_le_bytes(id_buf);
                                                Some(Command::GetResult { id })
                                            } else {
                                                None
                                            }
                                        }
                                        3 => {
                                            if payload.len() == 9 {
                                                let mut id_buf = [0u8; 8];
                                                id_buf.copy_from_slice(&payload[1..9]);
                                                let id = u64::from_le_bytes(id_buf);
                                                Some(Command::Cancel { id })
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    };

                                    if let Some(cmd) = req {
                                        return vec![WireMsg {
                                            client_id,
                                            command: cmd,
                                            uid: conn.uid,
                                        }];
                                    } else {
                                        should_disconnect = true;
                                        break;
                                    }
                                } else {
                                    should_disconnect = true;
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }

        if event.writable
            && !should_disconnect
            && let Some(conn) = self.clients.get_mut(&client_id)
        {
            while !conn.write_buf.is_empty() {
                match conn.fd.write(conn.write_buf.as_ptr(), conn.write_buf.len()) {
                    Ok(0) => {
                        should_disconnect = true;
                        break;
                    }
                    Ok(n) => {
                        conn.write_buf.drain(..n);
                    }
                    Err(e) => {
                        let raw_err = e.raw_os_error();
                        if raw_err == Some(libc::EAGAIN) || raw_err == Some(libc::EWOULDBLOCK) {
                            break;
                        } else {
                            should_disconnect = true;
                            break;
                        }
                    }
                }
            }
        }

        if should_disconnect {
            self.disconnect(client_id, reactor);
        }

        Vec::new()
    }

    pub fn disconnect(&mut self, client_id: u32, reactor: &mut crate::low_level::reactor::Reactor) {
        if let Some(conn) = self.clients.remove(&client_id) {
            reactor.del(&conn.fd);
            self.client_tokens.remove(&conn.token);
        }
    }

    pub fn intercept_action(&mut self, action: &crate::core::Action, reply_to: Option<u32>) {
        let client_id = match reply_to {
            Some(id) => id,
            None => return,
        };
        match action {
            crate::core::Action::Started { id } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Exec(*id));
                }
            }
            crate::core::Action::Controlled { id: _ } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::CancelOk);
                }
            }
            crate::core::Action::QueryResult { id: _, result } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Result(result.clone()));
                }
            }
            crate::core::Action::Rejected { .. } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Error);
                }
            }
            crate::core::Action::Finished { id, result, .. } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    let outcome = crate::core::ExecOutcome {
                        id: *id,
                        result: result.clone(),
                    };
                    Self::queue_response(conn, WireResponse::Result(Some(outcome)));
                }
            }
            _ => {}
        }
    }

    fn queue_response(conn: &mut Conn, resp: WireResponse) {
        if conn.write_buf.len() > MAX_WRITE_BUF {
            return; // Drop response on buffer overflow
        }

        let payload = match resp {
            WireResponse::Exec(id) => {
                let mut p = Vec::with_capacity(9);
                p.push(1u8);
                p.extend_from_slice(&id.to_le_bytes());
                p
            }
            WireResponse::Result(res) => {
                let mut p = Vec::with_capacity(1024);
                p.push(2u8);
                let json = serde_json::to_vec(&res).unwrap_or_default();
                p.extend_from_slice(&json);
                p
            }
            WireResponse::CancelOk => vec![3u8],
            WireResponse::Error => vec![4u8],
        };
        let len = payload.len() as u32;
        conn.write_buf.extend_from_slice(&len.to_le_bytes());
        conn.write_buf.extend_from_slice(&payload);
    }
}

use crate::high_level::api::Command;

enum WireResponse {
    Exec(u64),
    Result(Option<ExecOutcome>),
    CancelOk,
    Error,
}

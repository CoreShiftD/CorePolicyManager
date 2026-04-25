# IPC Protocol Reference

CoreShift provides a streaming UNIX domain socket for communication.

## Socket Configuration

- **Path**: `/data/local/tmp/coreshift/coreshift.sock` (from `paths::SOCKET_PATH`).
- **Authentication**: Uses `SO_PEERCRED` to securely verify the client's UID.

## Framing Format

Messages are length-prefixed:
- **Prefix**: 4-byte Little-Endian unsigned integer (`u32`) indicating the length of the following payload.

## Size Limits and Backpressure

To prevent unbounded memory growth, the daemon enforces strict sizes:
- `MAX_CLIENTS` = 32
- `MAX_PACKET_SIZE` = 128 KB
- `MAX_READ_BUF` = 256 KB
- `MAX_WRITE_BUF` = 1 MB (1024 * 1024)

*Disconnect Condition*: If a client's write buffer exceeds `MAX_WRITE_BUF` when queuing a response, or if a read exceeds bounds, the client is deliberately dropped.

## Request & Response Payloads

Payloads are prefixed with a 1-byte command identifier.

### Client Requests (to Daemon)
1. **JSON Command (Type `1`)**: The remainder of the payload is a JSON encoded `Command` enum (e.g., `Cmd { service, args }` or `Dumpsys { service, args }`).
2. **GetResult (Type `2`)**: Expects an exactly 8-byte Little-Endian `u64` ID.
3. **Cancel (Type `3`)**: Expects an exactly 8-byte Little-Endian `u64` ID.

### Daemon Responses (to Client)
1. **Exec (Type `1`)**: Returns the created job ID as an 8-byte Little-Endian `u64`.
2. **Result (Type `2`)**: Returns a JSON encoded `Option<ExecOutcome>`.
3. **CancelOk (Type `3`)**: No payload.
4. **Error (Type `4`)**: No payload.

> [!NOTE]
> **Experimental / Internal Protocol**: The IPC protocol structure is currently internal and subject to change.

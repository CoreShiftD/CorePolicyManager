package com.corepolicy.manager.core.data.bridge

import com.corepolicy.manager.core.domain.DaemonTransport

class LocalDaemonTransport : DaemonTransport {
    override suspend fun requestStatus(): String {
        return "Daemon channel reserved for local service or IPC transport."
    }
}

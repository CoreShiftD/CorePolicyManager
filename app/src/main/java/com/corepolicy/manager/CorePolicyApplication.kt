package com.corepolicy.manager

import android.app.Application
import com.corepolicy.manager.core.data.CorePolicyContainer

class CorePolicyApplication : Application() {
    val container by lazy { CorePolicyContainer(this) }
}

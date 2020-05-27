package com.actyx.os.android.service

import com.actyx.os.android.model.ActyxOsSettings
import io.reactivex.Single

interface Service : (ActyxOsSettings) -> Single<Unit>

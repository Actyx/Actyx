package com.actyx.android.util

import org.slf4j.Logger
import org.slf4j.LoggerFactory

inline fun <reified T> T.Logger(): Logger = LoggerFactory.getLogger(T::class.java)

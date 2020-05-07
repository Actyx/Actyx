package com.actyx.os.android.util

import org.slf4j.Logger
import org.slf4j.LoggerFactory

inline fun <reified T> T.Logger(): Logger = LoggerFactory.getLogger(T::class.java)
inline fun <T> T.Logger(tag: String): Logger = LoggerFactory.getLogger(tag)

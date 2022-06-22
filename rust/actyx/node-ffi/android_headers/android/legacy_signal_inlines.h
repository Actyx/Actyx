/*
 * Copyright (C) 2015 The Android Open Source Project
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *  * Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 *  * Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in
 *    the documentation and/or other materials provided with the
 *    distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

#ifndef _ANDROID_LEGACY_SIGNAL_INLINES_H_
#define _ANDROID_LEGACY_SIGNAL_INLINES_H_

#include <sys/cdefs.h>

#if __ANDROID_API__ < __ANDROID_API_L__

#include <errno.h>
#include <signal.h>
#include <string.h>

__BEGIN_DECLS

sighandler_t bsd_signal(int __signal, sighandler_t __handler) __REMOVED_IN(21);

/* These weren't introduced until L. */
int __libc_current_sigrtmax() __attribute__((__weak__)) __VERSIONER_NO_GUARD;
int __libc_current_sigrtmin() __attribute__((__weak__)) __VERSIONER_NO_GUARD;

static __inline int __ndk_legacy___libc_current_sigrtmax() {
  if (__libc_current_sigrtmax) return __libc_current_sigrtmax();
  return __SIGRTMAX; /* Should match __libc_current_sigrtmax. */
}

static __inline int __ndk_legacy___libc_current_sigrtmin() {
  if (__libc_current_sigrtmin) return __libc_current_sigrtmin();
  return __SIGRTMIN + 7; /* Should match __libc_current_sigrtmin. */
}

#undef SIGRTMAX
#define SIGRTMAX __ndk_legacy___libc_current_sigrtmax()
#undef SIGRTMIN
#define SIGRTMIN __ndk_legacy___libc_current_sigrtmin()

static __inline int sigismember(const sigset_t *set, int signum) {
  /* Signal numbers start at 1, but bit positions start at 0. */
  int bit = signum - 1;
  const unsigned long *local_set = (const unsigned long *)set;
  if (set == NULL || bit < 0 || bit >= (int)(8 * sizeof(sigset_t))) {
    errno = EINVAL;
    return -1;
  }
  return (int)((local_set[bit / LONG_BIT] >> (bit % LONG_BIT)) & 1);
}

static __inline int sigaddset(sigset_t *set, int signum) {
  /* Signal numbers start at 1, but bit positions start at 0. */
  int bit = signum - 1;
  unsigned long *local_set = (unsigned long *)set;
  if (set == NULL || bit < 0 || bit >= (int)(8 * sizeof(sigset_t))) {
    errno = EINVAL;
    return -1;
  }
  local_set[bit / LONG_BIT] |= 1UL << (bit % LONG_BIT);
  return 0;
}

static __inline int sigdelset(sigset_t *set, int signum) {
  /* Signal numbers start at 1, but bit positions start at 0. */
  int bit = signum - 1;
  unsigned long *local_set = (unsigned long *)set;
  if (set == NULL || bit < 0 || bit >= (int)(8 * sizeof(sigset_t))) {
    errno = EINVAL;
    return -1;
  }
  local_set[bit / LONG_BIT] &= ~(1UL << (bit % LONG_BIT));
  return 0;
}

static __inline int sigemptyset(sigset_t *set) {
  if (set == NULL) {
    errno = EINVAL;
    return -1;
  }
  memset(set, 0, sizeof(sigset_t));
  return 0;
}

static __inline int sigfillset(sigset_t *set) {
  if (set == NULL) {
    errno = EINVAL;
    return -1;
  }
  memset(set, ~0, sizeof(sigset_t));
  return 0;
}

static __inline sighandler_t signal(int s, sighandler_t f) {
  return bsd_signal(s, f);
}

__END_DECLS

#endif /* __ANDROID_API__ < __ANDROID_API_L__ */

#endif /* _ANDROID_LEGACY_SIGNAL_INLINES_H_ */

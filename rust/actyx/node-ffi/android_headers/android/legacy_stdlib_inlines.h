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

#ifndef _ANDROID_LEGACY_STDLIB_INLINES_H_
#define _ANDROID_LEGACY_STDLIB_INLINES_H_

#include <sys/cdefs.h>

#if __ANDROID_API__ < __ANDROID_API_K__

__BEGIN_DECLS

static __inline int abs(int __n) { return (__n < 0) ? -__n : __n; }

static __inline long labs(long __n) { return (__n < 0L) ? -__n : __n; }

static __inline long long llabs(long long __n) {
  return (__n < 0LL) ? -__n : __n;
}

__END_DECLS

#endif  /* __ANDROID_API__ < __ANDROID_API_K__ */

#if __ANDROID_API__ < __ANDROID_API_L__

#include <errno.h>
#include <float.h>
#include <stdlib.h>

__BEGIN_DECLS

static __inline float strtof(const char* nptr, char** endptr) {
  double d = strtod(nptr, endptr);
  if (d > FLT_MAX) {
    errno = ERANGE;
    return __builtin_huge_valf();
  } else if (d < -FLT_MAX) {
    errno = ERANGE;
    return -__builtin_huge_valf();
  }
  return __BIONIC_CAST(static_cast, float, d);
}

static __inline double atof(const char *nptr) { return (strtod(nptr, NULL)); }

static __inline int rand(void) { return (int)lrand48(); }

static __inline void srand(unsigned int __s) { srand48(__s); }

static __inline long random(void) { return lrand48(); }

static __inline void srandom(unsigned int __s) { srand48(__s); }

static __inline int grantpt(int __fd __attribute((unused))) {
  return 0; /* devpts does this all for us! */
}

__END_DECLS

#endif  /* __ANDROID_API__ < __ANDROID_API_L__ */

#if __ANDROID_API__ < __ANDROID_API_O__

#include <stdlib.h>
#include <xlocale.h>

__BEGIN_DECLS

static __inline double strtod_l(const char* __s, char** __end_ptr, locale_t __l) {
  return strtod(__s, __end_ptr);
}

static __inline float strtof_l(const char* __s, char** __end_ptr, locale_t __l) {
  return strtof(__s, __end_ptr);
}

static __inline long strtol_l(const char* __s, char** __end_ptr, int __base, locale_t __l) {
  return strtol(__s, __end_ptr, __base);
}

__END_DECLS

#endif  /* __ANDROID_API__ < __ANDROID_API_O__ */

#endif /* _ANDROID_LEGACY_STDLIB_INLINES_H_ */

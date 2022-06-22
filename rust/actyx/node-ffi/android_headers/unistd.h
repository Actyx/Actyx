/*
 * Copyright (C) 2008 The Android Open Source Project
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

#pragma once

#include <stddef.h>
#include <sys/cdefs.h>
#include <sys/types.h>
#include <sys/select.h>

#include <bits/fcntl.h>
#include <bits/getopt.h>
#include <bits/ioctl.h>
#include <bits/lockf.h>
#include <bits/posix_limits.h>
#include <bits/seek_constants.h>
#include <bits/sysconf.h>

__BEGIN_DECLS

#define STDIN_FILENO	0
#define STDOUT_FILENO	1
#define STDERR_FILENO	2

#define F_OK 0
#define X_OK 1
#define W_OK 2
#define R_OK 4

#define _PC_FILESIZEBITS 0
#define _PC_LINK_MAX 1
#define _PC_MAX_CANON 2
#define _PC_MAX_INPUT 3
#define _PC_NAME_MAX 4
#define _PC_PATH_MAX 5
#define _PC_PIPE_BUF 6
#define _PC_2_SYMLINKS 7
#define _PC_ALLOC_SIZE_MIN 8
#define _PC_REC_INCR_XFER_SIZE 9
#define _PC_REC_MAX_XFER_SIZE 10
#define _PC_REC_MIN_XFER_SIZE 11
#define _PC_REC_XFER_ALIGN 12
#define _PC_SYMLINK_MAX 13
#define _PC_CHOWN_RESTRICTED 14
#define _PC_NO_TRUNC 15
#define _PC_VDISABLE 16
#define _PC_ASYNC_IO 17
#define _PC_PRIO_IO 18
#define _PC_SYNC_IO 19

extern char** environ;

__noreturn void _exit(int __status);

pid_t  fork(void);
pid_t  vfork(void) __returns_twice;
pid_t  getpid(void);
pid_t  gettid(void) __attribute_const__;
pid_t  getpgid(pid_t __pid);
int    setpgid(pid_t __pid, pid_t __pgid);
pid_t  getppid(void);
pid_t  getpgrp(void);
int    setpgrp(void);

#if __ANDROID_API__ >= 17
pid_t  getsid(pid_t __pid) __INTRODUCED_IN(17);
#endif /* __ANDROID_API__ >= 17 */

pid_t  setsid(void);

int execv(const char* __path, char* const* __argv);
int execvp(const char* __file, char* const* __argv);

#if __ANDROID_API__ >= 21
int execvpe(const char* __file, char* const* __argv, char* const* __envp) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

int execve(const char* __file, char* const* __argv, char* const* __envp);
int execl(const char* __path, const char* __arg0, ...) __attribute__((__sentinel__));
int execlp(const char* __file, const char* __arg0, ...) __attribute__((__sentinel__));
int execle(const char* __path, const char* __arg0, ... /*,  char* const* __envp */)
    __attribute__((__sentinel__(1)));

#if __ANDROID_API__ >= 28
int fexecve(int __fd, char* const* __argv, char* const* __envp) __INTRODUCED_IN(28);
#endif /* __ANDROID_API__ >= 28 */


int nice(int __incr);

/**
 * [setegid(2)](http://man7.org/linux/man-pages/man2/setegid.2.html) sets
 * the effective group ID.
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setegid(gid_t __gid);

/**
 * [seteuid(2)](http://man7.org/linux/man-pages/man2/seteuid.2.html) sets
 * the effective user ID.
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int seteuid(uid_t __uid);

/**
 * [setgid(2)](http://man7.org/linux/man-pages/man2/setgid.2.html) sets
 * the group ID.
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setgid(gid_t __gid);

/**
 * [setregid(2)](http://man7.org/linux/man-pages/man2/setregid.2.html) sets
 * the real and effective group IDs (use -1 to leave an ID unchanged).
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setregid(gid_t __rgid, gid_t __egid);

/**
 * [setresgid(2)](http://man7.org/linux/man-pages/man2/setresgid.2.html) sets
 * the real, effective, and saved group IDs (use -1 to leave an ID unchanged).
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setresgid(gid_t __rgid, gid_t __egid, gid_t __sgid);

/**
 * [setresuid(2)](http://man7.org/linux/man-pages/man2/setresuid.2.html) sets
 * the real, effective, and saved user IDs (use -1 to leave an ID unchanged).
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setresuid(uid_t __ruid, uid_t __euid, uid_t __suid);

/**
 * [setreuid(2)](http://man7.org/linux/man-pages/man2/setreuid.2.html) sets
 * the real and effective group IDs (use -1 to leave an ID unchanged).
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setreuid(uid_t __ruid, uid_t __euid);

/**
 * [setuid(2)](http://man7.org/linux/man-pages/man2/setuid.2.html) sets
 * the user ID.
 *
 * On Android, this function only affects the calling thread, not all threads
 * in the process.
 *
 * Returns 0 on success, and returns -1 and sets `errno` on failure.
 */
int setuid(uid_t __uid);

uid_t getuid(void);
uid_t geteuid(void);
gid_t getgid(void);
gid_t getegid(void);
int getgroups(int __size, gid_t* __list);
int setgroups(size_t __size, const gid_t* __list);
int getresuid(uid_t* __ruid, uid_t* __euid, uid_t* __suid);
int getresgid(gid_t* __rgid, gid_t* __egid, gid_t* __sgid);
char* getlogin(void);

#if __ANDROID_API__ >= 28
int getlogin_r(char* __buffer, size_t __buffer_size) __INTRODUCED_IN(28);
#endif /* __ANDROID_API__ >= 28 */


long fpathconf(int __fd, int __name);
long pathconf(const char* __path, int __name);

int access(const char* __path, int __mode);
int faccessat(int __dirfd, const char* __path, int __mode, int __flags);
int link(const char* __old_path, const char* __new_path);

#if __ANDROID_API__ >= 21
int linkat(int __old_dir_fd, const char* __old_path, int __new_dir_fd, const char* __new_path, int __flags) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

int unlink(const char* __path);
int unlinkat(int __dirfd, const char* __path, int __flags);
int chdir(const char* __path);
int fchdir(int __fd);
int rmdir(const char* __path);
int pipe(int __fds[2]);
#if defined(__USE_GNU)
int pipe2(int __fds[2], int __flags);
#endif
int chroot(const char* __path);
int symlink(const char* __old_path, const char* __new_path);

#if __ANDROID_API__ >= 21
int symlinkat(const char* __old_path, int __new_dir_fd, const char* __new_path) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

ssize_t readlink(const char* __path, char* __buf, size_t __buf_size);

#if __ANDROID_API__ >= 21
ssize_t readlinkat(int __dir_fd, const char* __path, char* __buf, size_t __buf_size)
    __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

int chown(const char* __path, uid_t __owner, gid_t __group);
int fchown(int __fd, uid_t __owner, gid_t __group);
int fchownat(int __dir_fd, const char* __path, uid_t __owner, gid_t __group, int __flags);
int lchown(const char* __path, uid_t __owner, gid_t __group);
char* getcwd(char* __buf, size_t __size);

void sync(void);
#if defined(__USE_GNU)

#if __ANDROID_API__ >= 28
int syncfs(int __fd) __INTRODUCED_IN(28);
#endif /* __ANDROID_API__ >= 28 */

#endif

int close(int __fd);

ssize_t read(int __fd, void* __buf, size_t __count);
ssize_t write(int __fd, const void* __buf, size_t __count);

int dup(int __old_fd);
int dup2(int __old_fd, int __new_fd);

#if __ANDROID_API__ >= 21
int dup3(int __old_fd, int __new_fd, int __flags) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

int fsync(int __fd);
int fdatasync(int __fd);

/* See https://android.googlesource.com/platform/bionic/+/master/docs/32-bit-abi.md */
#if defined(__USE_FILE_OFFSET64)

#if __ANDROID_API__ >= 21
int truncate(const char* __path, off_t __length) __RENAME(truncate64) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

off_t lseek(int __fd, off_t __offset, int __whence) __RENAME(lseek64);
ssize_t pread(int __fd, void* __buf, size_t __count, off_t __offset) __RENAME(pread64);
ssize_t pwrite(int __fd, const void* __buf, size_t __count, off_t __offset) __RENAME(pwrite64);
int ftruncate(int __fd, off_t __length) __RENAME(ftruncate64);
#else
int truncate(const char* __path, off_t __length);
off_t lseek(int __fd, off_t __offset, int __whence);
ssize_t pread(int __fd, void* __buf, size_t __count, off_t __offset);
ssize_t pwrite(int __fd, const void* __buf, size_t __count, off_t __offset);
int ftruncate(int __fd, off_t __length);
#endif


#if __ANDROID_API__ >= 21
int truncate64(const char* __path, off64_t __length) __INTRODUCED_IN(21);
#endif /* __ANDROID_API__ >= 21 */

off64_t lseek64(int __fd, off64_t __offset, int __whence);
ssize_t pread64(int __fd, void* __buf, size_t __count, off64_t __offset);
ssize_t pwrite64(int __fd, const void* __buf, size_t __count, off64_t __offset);
int ftruncate64(int __fd, off64_t __length);

int pause(void);
unsigned int alarm(unsigned int __seconds);
unsigned int sleep(unsigned int __seconds);
int usleep(useconds_t __microseconds);

int gethostname(char* __buf, size_t __buf_size);

#if __ANDROID_API__ >= 23
int sethostname(const char* __name, size_t __n) __INTRODUCED_IN(23);
#endif /* __ANDROID_API__ >= 23 */


int brk(void* __addr);
void* sbrk(ptrdiff_t __increment);

int isatty(int __fd);
char* ttyname(int __fd);
int ttyname_r(int __fd, char* __buf, size_t __buf_size);

int acct(const char* __path);

#if __ANDROID_API__ >= __ANDROID_API_L__
int getpagesize(void) __INTRODUCED_IN(21);
#else
static __inline__ int getpagesize(void) {
  return sysconf(_SC_PAGESIZE);
}
#endif

long syscall(long __number, ...);

int daemon(int __no_chdir, int __no_close);

#if defined(__arm__) || (defined(__mips__) && !defined(__LP64__))
int cacheflush(long __addr, long __nbytes, long __cache);
    /* __attribute__((deprecated("use __builtin___clear_cache instead"))); */
#endif

pid_t tcgetpgrp(int __fd);
int tcsetpgrp(int __fd, pid_t __pid);

/* Used to retry syscalls that can return EINTR. */
#define TEMP_FAILURE_RETRY(exp) ({         \
    __typeof__(exp) _rc;                   \
    do {                                   \
        _rc = (exp);                       \
    } while (_rc == -1 && errno == EINTR); \
    _rc; })


#if __ANDROID_API__ >= 26
int getdomainname(char* __buf, size_t __buf_size) __INTRODUCED_IN(26);
int setdomainname(const char* __name, size_t __n) __INTRODUCED_IN(26);
#endif /* __ANDROID_API__ >= 26 */



#if __ANDROID_API__ >= 28
void swab(const void* __src, void* __dst, ssize_t __byte_count) __INTRODUCED_IN(28);
#endif /* __ANDROID_API__ >= 28 */


#if defined(__BIONIC_INCLUDE_FORTIFY_HEADERS)
#define _UNISTD_H_
#include <bits/fortify/unistd.h>
#undef _UNISTD_H_
#endif

__END_DECLS

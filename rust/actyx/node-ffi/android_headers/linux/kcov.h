/****************************************************************************
 ****************************************************************************
 ***
 ***   This header was automatically generated from a Linux kernel header
 ***   of the same name, to make information necessary for userspace to
 ***   call into the kernel available to libc.  It contains only constants,
 ***   structures, and macros generated from the original header, and thus,
 ***   contains no copyrightable information.
 ***
 ***   To edit the content of this header, modify the corresponding
 ***   source file (e.g. under external/kernel-headers/original/) then
 ***   run bionic/libc/kernel/tools/update_all.py
 ***
 ***   Any manual change here will be lost the next time this script will
 ***   be run. You've been warned!
 ***
 ****************************************************************************
 ****************************************************************************/
#ifndef _LINUX_KCOV_IOCTLS_H
#define _LINUX_KCOV_IOCTLS_H
#include <linux/types.h>
#define KCOV_INIT_TRACE _IOR('c', 1, unsigned long)
#define KCOV_ENABLE _IO('c', 100)
#define KCOV_DISABLE _IO('c', 101)
enum {
  KCOV_TRACE_PC = 0,
  KCOV_TRACE_CMP = 1,
};
#define KCOV_CMP_CONST (1 << 0)
#define KCOV_CMP_SIZE(n) ((n) << 1)
#define KCOV_CMP_MASK KCOV_CMP_SIZE(3)
#endif

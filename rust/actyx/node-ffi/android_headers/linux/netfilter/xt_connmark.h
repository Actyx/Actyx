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
#ifndef _XT_CONNMARK_H
#define _XT_CONNMARK_H
#include <linux/types.h>
enum {
  XT_CONNMARK_SET = 0,
  XT_CONNMARK_SAVE,
  XT_CONNMARK_RESTORE
};
enum {
  D_SHIFT_LEFT = 0,
  D_SHIFT_RIGHT,
};
struct xt_connmark_tginfo1 {
  __u32 ctmark, ctmask, nfmask;
  __u8 mode;
};
struct xt_connmark_tginfo2 {
  __u32 ctmark, ctmask, nfmask;
  __u8 shift_dir, shift_bits, mode;
};
struct xt_connmark_mtinfo1 {
  __u32 mark, mask;
  __u8 invert;
};
#endif

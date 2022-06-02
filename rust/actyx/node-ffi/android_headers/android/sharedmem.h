/*
 * Copyright (C) 2017 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * @addtogroup Memory
 * @{
 */

/**
 * @file sharedmem.h
 * @brief Shared memory buffers that can be shared between processes.
 */

#ifndef ANDROID_SHARED_MEMORY_H
#define ANDROID_SHARED_MEMORY_H

#include <stddef.h>
#include <sys/cdefs.h>

/******************************************************************
 *
 * IMPORTANT NOTICE:
 *
 *   This file is part of Android's set of stable system headers
 *   exposed by the Android NDK (Native Development Kit).
 *
 *   Third-party source AND binary code relies on the definitions
 *   here to be FROZEN ON ALL UPCOMING PLATFORM RELEASES.
 *
 *   - DO NOT MODIFY ENUMS (EXCEPT IF YOU ADD NEW 32-BIT VALUES)
 *   - DO NOT MODIFY CONSTANTS OR FUNCTIONAL MACROS
 *   - DO NOT CHANGE THE SIGNATURE OF FUNCTIONS IN ANY WAY
 *   - DO NOT CHANGE THE LAYOUT OR SIZE OF STRUCTURES
 */

#ifdef __cplusplus
extern "C" {
#endif

#if __ANDROID_API__ >= 26

/**
 * Create a shared memory region.
 *
 * Create shared memory region and returns an file descriptor.  The resulting file descriptor can be
 * mmap'ed to process memory space with PROT_READ | PROT_WRITE | PROT_EXEC. Access to shared memory
 * region can be restricted with {@link ASharedMemory_setProt}.
 *
 * Use close() to release the shared memory region.
 *
 * Use {@link android.os.ParcelFileDescriptor} to pass the file descriptor to
 * another process. File descriptors may also be sent to other processes over a Unix domain
 * socket with sendmsg and SCM_RIGHTS. See sendmsg(3) and cmsg(3) man pages for more information.
 *
 * Available since API level 26.
 *
 * \param name an optional name.
 * \param size size of the shared memory region
 * \return file descriptor that denotes the shared memory; -1 and sets errno on failure, or -EINVAL if the error is that size was 0.
 */
int ASharedMemory_create(const char *name, size_t size) __INTRODUCED_IN(26);

/**
 * Get the size of the shared memory region.
 *
 * Available since API level 26.
 *
 * \param fd file descriptor of the shared memory region
 * \return size in bytes; 0 if fd is not a valid shared memory file descriptor.
 */
size_t ASharedMemory_getSize(int fd) __INTRODUCED_IN(26);

/**
 * Restrict access of shared memory region.
 *
 * This function restricts access of a shared memory region. Access can only be removed. The effect
 * applies globally to all file descriptors in all processes across the system that refer to this
 * shared memory region. Existing memory mapped regions are not affected.
 *
 * It is a common use case to create a shared memory region, map it read/write locally to intialize
 * content, and then send the shared memory to another process with read only access. Code example
 * as below (error handling omited).
 *
 *
 *     int fd = ASharedMemory_create("memory", 128);
 *
 *     // By default it has PROT_READ | PROT_WRITE | PROT_EXEC.
 *     size_t memSize = ASharedMemory_getSize(fd);
 *     char *buffer = (char *) mmap(NULL, memSize, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
 *
 *     strcpy(buffer, "This is an example."); // trivially initialize content
 *
 *     // limit access to read only
 *     ASharedMemory_setProt(fd, PROT_READ);
 *
 *     // share fd with another process here and the other process can only map with PROT_READ.
 *
 * Available since API level 26.
 *
 * \param fd   file descriptor of the shared memory region.
 * \param prot any bitwise-or'ed combination of PROT_READ, PROT_WRITE, PROT_EXEC denoting
 *             updated access. Note access can only be removed, but not added back.
 * \return 0 for success, -1 and sets errno on failure.
 */
int ASharedMemory_setProt(int fd, int prot) __INTRODUCED_IN(26);

#endif // __ANDROID_API__ >= 26

#ifdef __cplusplus
};
#endif

#endif // ANDROID_SHARED_MEMORY_H

/** @} */

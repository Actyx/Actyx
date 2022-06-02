/*
 * Copyright (C) 2018 The Android Open Source Project
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
 * @addtogroup SurfaceTexture
 * @{
 */

/**
 * @file surface_texture_jni.h
 */

#ifndef ANDROID_NATIVE_SURFACE_TEXTURE_JNI_H
#define ANDROID_NATIVE_SURFACE_TEXTURE_JNI_H

#include <android/surface_texture.h>

#include <jni.h>

__BEGIN_DECLS

#if __ANDROID_API__ >= 28

/**
 * Get a reference to the native ASurfaceTexture from the corresponding java object.
 *
 * The caller must keep a reference to the Java SurfaceTexture during the lifetime of the returned
 * ASurfaceTexture. Failing to do so could result in the ASurfaceTexture to stop functioning
 * properly once the Java object gets finalized.
 * However, this will not result in program termination.
 *
 * Available since API level 28.
 *
 * \param env JNI environment
 * \param surfacetexture Instance of Java SurfaceTexture object
 * \return native ASurfaceTexture reference or nullptr if the java object is not a SurfaceTexture.
 *         The returned reference MUST BE released when it's no longer needed using
 *         ASurfaceTexture_release().
 */
ASurfaceTexture* ASurfaceTexture_fromSurfaceTexture(JNIEnv* env, jobject surfacetexture) __INTRODUCED_IN(28);

#endif

__END_DECLS

#endif /* ANDROID_NATIVE_SURFACE_TEXTURE_JNI_H */

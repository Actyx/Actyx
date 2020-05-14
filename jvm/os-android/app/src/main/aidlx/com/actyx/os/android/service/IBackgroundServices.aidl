package com.actyx.os.android.service;

import com.actyx.os.android.AppInfo;

// FIXME figure out how to set the output dir for the compile{Release,Debug}Aidl task because by
// default it writes to
// build/generated/aidl_source_output_dir/debug/compileDebugAidl/out/<absolute project path>
// /android-actyxos-app/app/src/main/aidl/
// see https://issuetracker.google.com/issues/150151300
// If you need to update the aidl files, rename the `aidlx` folder to `aidl`, run the
// `compileReleaseAidl` task and replace the existing `IBackgroundServices.java` file
// Also make sure to apply this diff (https://youtrack.jetbrains.com/issue/KT-25807):
// @@ -156,7 +183,7 @@ public interface IBackgroundServices extends android.os.IInterface
   //           }
   //           _reply.readException();
   //           if ((0!=_reply.readInt())) {
   //-            _result = (com.actyx.os.android.AppInfo)com.actyx.os.android.AppInfo.CREATOR.createFromParcel(_reply);
   //+            _result = com.actyx.os.android.AppInfo.CREATOR.createFromParcel(_reply);
   //           }
   //           else {
   //             _result = null;
interface IBackgroundServices {
  List<AppInfo> getApps();
  AppInfo getAppInfo(String appId);
  String getSettings(String scope);
  void onAppStarted(String appId);
  void onAppStopped(String appId);
  void onAppEnabled(String appId);
  void onAppDisabled(String appId);
}

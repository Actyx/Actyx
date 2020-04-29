package com.actyx.os.android.viewmodel

import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.ViewModel
import com.actyx.os.android.AppInfo

class AppInfosViewModel : ViewModel() {
  private val appInfos: MutableLiveData<List<AppInfo>> =
    MutableLiveData<List<AppInfo>>()

  fun getAppInfos(): LiveData<List<AppInfo>> {
    return appInfos
  }

  // TODO: inject apps repository into this view model,
  //  delete this method and clean up main activity
  fun setAppInfos(apps: List<AppInfo>) {
    appInfos.value = apps
  }
}

/*
 * This file is auto-generated.  DO NOT MODIFY.
 */
package com.actyx.os.android.service;
// FIXME figure out how to set the output dir for the compile{Release,Debug}Aidl task because by
// default it writes to
// build/generated/aidl_source_output_dir/debug/compileDebugAidl/out/<absolute project path>
// /android-actyxos-app/app/src/main/aidl/
// see https://issuetracker.google.com/issues/150151300
// If you need to update the aidl files, rename the `aidlx` folder to `aidl`, run the
// `compileReleaseAidl` task and replace the existing `IBackgroundServices.java` file

public interface IBackgroundServices extends android.os.IInterface
{
  /** Default implementation for IBackgroundServices. */
  public static class Default implements com.actyx.os.android.service.IBackgroundServices
  {
    @Override public java.util.List<com.actyx.os.android.AppInfo> getApps() throws android.os.RemoteException
    {
      return null;
    }
    @Override public com.actyx.os.android.AppInfo getAppInfo(java.lang.String appId) throws android.os.RemoteException
    {
      return null;
    }
    @Override public java.lang.String getSettings(java.lang.String scope) throws android.os.RemoteException
    {
      return null;
    }
    @Override public void onAppStarted(java.lang.String appId) throws android.os.RemoteException
    {
    }
    @Override public void onAppStopped(java.lang.String appId) throws android.os.RemoteException
    {
    }
    @Override
    public android.os.IBinder asBinder() {
      return null;
    }
  }
  /** Local-side IPC implementation stub class. */
  public static abstract class Stub extends android.os.Binder implements com.actyx.os.android.service.IBackgroundServices
  {
    private static final java.lang.String DESCRIPTOR = "com.actyx.os.android.service.IBackgroundServices";
    /** Construct the stub at attach it to the interface. */
    public Stub()
    {
      this.attachInterface(this, DESCRIPTOR);
    }
    /**
     * Cast an IBinder object into an com.actyx.os.android.service.IBackgroundServices interface,
     * generating a proxy if needed.
     */
    public static com.actyx.os.android.service.IBackgroundServices asInterface(android.os.IBinder obj)
    {
      if ((obj==null)) {
        return null;
      }
      android.os.IInterface iin = obj.queryLocalInterface(DESCRIPTOR);
      if (((iin!=null)&&(iin instanceof com.actyx.os.android.service.IBackgroundServices))) {
        return ((com.actyx.os.android.service.IBackgroundServices)iin);
      }
      return new com.actyx.os.android.service.IBackgroundServices.Stub.Proxy(obj);
    }
    @Override public android.os.IBinder asBinder()
    {
      return this;
    }
    @Override public boolean onTransact(int code, android.os.Parcel data, android.os.Parcel reply, int flags) throws android.os.RemoteException
    {
      java.lang.String descriptor = DESCRIPTOR;
      switch (code)
      {
        case INTERFACE_TRANSACTION:
        {
          reply.writeString(descriptor);
          return true;
        }
        case TRANSACTION_getApps:
        {
          data.enforceInterface(descriptor);
          java.util.List<com.actyx.os.android.AppInfo> _result = this.getApps();
          reply.writeNoException();
          reply.writeTypedList(_result);
          return true;
        }
        case TRANSACTION_getAppInfo:
        {
          data.enforceInterface(descriptor);
          java.lang.String _arg0;
          _arg0 = data.readString();
          com.actyx.os.android.AppInfo _result = this.getAppInfo(_arg0);
          reply.writeNoException();
          if ((_result!=null)) {
            reply.writeInt(1);
            _result.writeToParcel(reply, android.os.Parcelable.PARCELABLE_WRITE_RETURN_VALUE);
          }
          else {
            reply.writeInt(0);
          }
          return true;
        }
        case TRANSACTION_getSettings:
        {
          data.enforceInterface(descriptor);
          java.lang.String _arg0;
          _arg0 = data.readString();
          java.lang.String _result = this.getSettings(_arg0);
          reply.writeNoException();
          reply.writeString(_result);
          return true;
        }
        case TRANSACTION_onAppStarted:
        {
          data.enforceInterface(descriptor);
          java.lang.String _arg0;
          _arg0 = data.readString();
          this.onAppStarted(_arg0);
          reply.writeNoException();
          return true;
        }
        case TRANSACTION_onAppStopped:
        {
          data.enforceInterface(descriptor);
          java.lang.String _arg0;
          _arg0 = data.readString();
          this.onAppStopped(_arg0);
          reply.writeNoException();
          return true;
        }
        default:
        {
          return super.onTransact(code, data, reply, flags);
        }
      }
    }
    private static class Proxy implements com.actyx.os.android.service.IBackgroundServices
    {
      private android.os.IBinder mRemote;
      Proxy(android.os.IBinder remote)
      {
        mRemote = remote;
      }
      @Override public android.os.IBinder asBinder()
      {
        return mRemote;
      }
      public java.lang.String getInterfaceDescriptor()
      {
        return DESCRIPTOR;
      }
      @Override public java.util.List<com.actyx.os.android.AppInfo> getApps() throws android.os.RemoteException
      {
        android.os.Parcel _data = android.os.Parcel.obtain();
        android.os.Parcel _reply = android.os.Parcel.obtain();
        java.util.List<com.actyx.os.android.AppInfo> _result;
        try {
          _data.writeInterfaceToken(DESCRIPTOR);
          boolean _status = mRemote.transact(Stub.TRANSACTION_getApps, _data, _reply, 0);
          if (!_status && getDefaultImpl() != null) {
            return getDefaultImpl().getApps();
          }
          _reply.readException();
          _result = _reply.createTypedArrayList(com.actyx.os.android.AppInfo.CREATOR);
        }
        finally {
          _reply.recycle();
          _data.recycle();
        }
        return _result;
      }
      @Override public com.actyx.os.android.AppInfo getAppInfo(java.lang.String appId) throws android.os.RemoteException
      {
        android.os.Parcel _data = android.os.Parcel.obtain();
        android.os.Parcel _reply = android.os.Parcel.obtain();
        com.actyx.os.android.AppInfo _result;
        try {
          _data.writeInterfaceToken(DESCRIPTOR);
          _data.writeString(appId);
          boolean _status = mRemote.transact(Stub.TRANSACTION_getAppInfo, _data, _reply, 0);
          if (!_status && getDefaultImpl() != null) {
            return getDefaultImpl().getAppInfo(appId);
          }
          _reply.readException();
          if ((0!=_reply.readInt())) {
            _result = (com.actyx.os.android.AppInfo)com.actyx.os.android.AppInfo.CREATOR.createFromParcel(_reply);
          }
          else {
            _result = null;
          }
        }
        finally {
          _reply.recycle();
          _data.recycle();
        }
        return _result;
      }
      @Override public java.lang.String getSettings(java.lang.String scope) throws android.os.RemoteException
      {
        android.os.Parcel _data = android.os.Parcel.obtain();
        android.os.Parcel _reply = android.os.Parcel.obtain();
        java.lang.String _result;
        try {
          _data.writeInterfaceToken(DESCRIPTOR);
          _data.writeString(scope);
          boolean _status = mRemote.transact(Stub.TRANSACTION_getSettings, _data, _reply, 0);
          if (!_status && getDefaultImpl() != null) {
            return getDefaultImpl().getSettings(scope);
          }
          _reply.readException();
          _result = _reply.readString();
        }
        finally {
          _reply.recycle();
          _data.recycle();
        }
        return _result;
      }
      @Override public void onAppStarted(java.lang.String appId) throws android.os.RemoteException
      {
        android.os.Parcel _data = android.os.Parcel.obtain();
        android.os.Parcel _reply = android.os.Parcel.obtain();
        try {
          _data.writeInterfaceToken(DESCRIPTOR);
          _data.writeString(appId);
          boolean _status = mRemote.transact(Stub.TRANSACTION_onAppStarted, _data, _reply, 0);
          if (!_status && getDefaultImpl() != null) {
            getDefaultImpl().onAppStarted(appId);
            return;
          }
          _reply.readException();
        }
        finally {
          _reply.recycle();
          _data.recycle();
        }
      }
      @Override public void onAppStopped(java.lang.String appId) throws android.os.RemoteException
      {
        android.os.Parcel _data = android.os.Parcel.obtain();
        android.os.Parcel _reply = android.os.Parcel.obtain();
        try {
          _data.writeInterfaceToken(DESCRIPTOR);
          _data.writeString(appId);
          boolean _status = mRemote.transact(Stub.TRANSACTION_onAppStopped, _data, _reply, 0);
          if (!_status && getDefaultImpl() != null) {
            getDefaultImpl().onAppStopped(appId);
            return;
          }
          _reply.readException();
        }
        finally {
          _reply.recycle();
          _data.recycle();
        }
      }
      public static com.actyx.os.android.service.IBackgroundServices sDefaultImpl;
    }
    static final int TRANSACTION_getApps = (android.os.IBinder.FIRST_CALL_TRANSACTION + 0);
    static final int TRANSACTION_getAppInfo = (android.os.IBinder.FIRST_CALL_TRANSACTION + 1);
    static final int TRANSACTION_getSettings = (android.os.IBinder.FIRST_CALL_TRANSACTION + 2);
    static final int TRANSACTION_onAppStarted = (android.os.IBinder.FIRST_CALL_TRANSACTION + 3);
    static final int TRANSACTION_onAppStopped = (android.os.IBinder.FIRST_CALL_TRANSACTION + 4);
    public static boolean setDefaultImpl(com.actyx.os.android.service.IBackgroundServices impl) {
      if (Stub.Proxy.sDefaultImpl == null && impl != null) {
        Stub.Proxy.sDefaultImpl = impl;
        return true;
      }
      return false;
    }
    public static com.actyx.os.android.service.IBackgroundServices getDefaultImpl() {
      return Stub.Proxy.sDefaultImpl;
    }
  }
  public java.util.List<com.actyx.os.android.AppInfo> getApps() throws android.os.RemoteException;
  public com.actyx.os.android.AppInfo getAppInfo(java.lang.String appId) throws android.os.RemoteException;
  public java.lang.String getSettings(java.lang.String scope) throws android.os.RemoteException;
  public void onAppStarted(java.lang.String appId) throws android.os.RemoteException;
  public void onAppStopped(java.lang.String appId) throws android.os.RemoteException;
}
